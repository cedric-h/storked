use crate::net::prelude::*;
use comn::{
    item::{DropRequest, Inventory, PickupRequest, MAX_INTERACTION_DISTANCE_SQUARED},
    prelude::*,
};
use log::*;
use specs::prelude::*;

/// This System processes requests from clients to pick things up.
pub struct ItemPickupDrop;
impl<'a> System<'a> for ItemPickupDrop {
    type SystemData = (
        Entities<'a>,
        Read<'a, ConnectionManager>,
        WriteStorage<'a, DropRequest>,
        WriteStorage<'a, PickupRequest>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Inventory>,
        ReadStorage<'a, Item>,
        ReadStorage<'a, Client>,
    );

    fn run(
        &mut self,
        (ents, cm, mut drops, mut picks, mut poses, mut invs, items, clients): Self::SystemData,
    ) {
        (&*ents, &poses, drops.drain())
            .join()
            .map(
                |(player_ent, player_pos, DropRequest { item_index })| -> (Entity, Pos, _) {
                    (player_ent, player_pos.clone(), item_index)
                },
            )
            .collect::<Vec<_>>()
            .into_iter()
            .for_each(|(player_ent, player_pos, item_inventory_index)| {
                info!("re-physicalizing an item!");

                // taking the item out of their inventory
                let player_inventory = invs.get_mut(player_ent).expect(
                    "Couldn't get inventory for player to drop item",
                );
                let item_ent = ents.entity(match player_inventory.clear(&item_inventory_index) {
                    Ok(slot) => match slot {
                        Some(ent) => ent,
                        None => {
                            // POSSIBLE HACKING
                            error!(
                                "Player[{}] attempted to drop item at invalid index: {:?}",
                                player_ent.id(),
                                item_inventory_index,
                            );
                            return;
                        }
                    },
                    Err(e) => {
                        // POSSIBLE HACKING
                        error!(
                            "Error fetching item in order to drop it. Player[{}], item index {:?}: {:?}",
                            player_ent.id(),
                            item_inventory_index,
                            e
                        );
                        return;
                    }
                });

                // re-physicalizing the item
                for &Client(addr) in clients.join() {
                    cm.insert_comp(addr, item_ent, player_pos.clone());
                }
                poses
                    .insert(item_ent, player_pos)
                    .expect("Couldn't insert position to re-physicalize an item");

                // updating the client's record of their player's inventory
                let &Client(player_addr) = clients.get(player_ent).expect(
                    "Couldn't get address for player to refresh their inventory after drop",
                );
                cm.insert_comp(player_addr, player_ent, player_inventory.clone());
            });

        (&*ents, &mut invs, picks.drain(), &poses, &clients)
            .join()
            // who the player wants to pick up and where the player is
            .filter_map(
                |(
                    player_ent,
                    player_inventory,
                    PickupRequest { id },
                    &Pos(Iso2 {
                        translation: p_trans,
                        ..
                    }),
                    &Client(player_addr),
                )| {
                    info!("got request");
                    let item_ent = ents.entity(id);
                    // get the pos of the item they want to pickup
                    // the question marks will prevent them from picking this up
                    // if the item in question doesn't have a position or item.
                    let &Pos(Iso2 {
                        translation: i_trans,
                        ..
                    }) = poses.get(item_ent)?;
                    let item_item = items.get(item_ent)?;
                    info!("passed requirements");

                    let player_to_item_distance_squared =
                        (p_trans.vector - i_trans.vector).magnitude_squared();

                    // actually close enough!
                    if player_to_item_distance_squared < MAX_INTERACTION_DISTANCE_SQUARED {
                        use comn::item::Error;
                        match player_inventory.insert(item_ent.id(), item_item) {
                            Err(e) => match e {
                                Error::InventoryFull => None,
                                other_e => {
                                    // The Inventory API massively fucked up
                                    error!(
                                        "Couldn't insert Item[{}] into Player[{}]'s inventory: {:?}",
                                        item_ent.id(),
                                        player_ent.id(),
                                        other_e,
                                    );
                                    None
                                }
                            }
                            index => {
                                info!(
                                    "Inserting Item[{}] into Player[{}]'s inventory at index: {:?}!",
                                    item_ent.id(),
                                    player_ent.id(),
                                    index,
                                );
                                cm.insert_comp(player_addr, player_ent, player_inventory.clone());
                                Some(item_ent)
                            },
                        }
                    } else {
                        // tryna hack!?
                        None
                    }
                },
            )
            .collect::<Vec<_>>()
            .into_iter()
            // the items are close enough! itemize all 'em mfers!
            .for_each(|item_ent| {
                poses
                    .remove(item_ent)
                    .expect("couldn't un-positionize an item to pick it up!");

                for &Client(addr) in clients.join() {
                    cm.insert_comp(addr, item_ent, comn::item::Deposition);
                }
            });
    }
}
