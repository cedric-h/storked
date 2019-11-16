// our code
use super::prelude::*;
use log::*;
// crates
use comn::{na, specs::prelude::*, Pos};

/// This system sends the world to all clients with the LoggingIn component.
pub struct SendWorldToNewPlayers;
impl<'a> System<'a> for SendWorldToNewPlayers {
    type SystemData = (
        // things we need to do networking
        Read<'a, ConnectionManager>,
        WriteStorage<'a, LoggingIn>,
        ReadStorage<'a, Client>,
        // things we need to tell new players about
        Entities<'a>,
        ReadStorage<'a, comn::art::Appearance>,
        ReadStorage<'a, comn::art::Tile>,
        ReadStorage<'a, comn::art::Animate>,
        ReadStorage<'a, Pos>,
    );

    fn run(
        &mut self,
        (cm, mut logging_ins, clients, ents, appearances, tiles, animates, isos): Self::SystemData,
    ) {
        for (_, Client(addr)) in (logging_ins.drain(), &clients).join() {
            debug!("We're about to tell a new player about the world.");
            // tell them about each new entity they need to add, and about
            // some crucial components it has.
            for (iso, appearance, tile, animate, ent) in (
                &isos,
                appearances.maybe(),
                tiles.maybe(),
                animates.maybe(),
                &*ents,
            )
                .join()
            {
                trace!("telling new player about an existing entity");
                cm.new_ent(*addr, ent);
                cm.insert_comp(*addr, ent, iso.clone());

                // I should really do all of these using some more macro
                // abomination on net_component
                if let Some(appearance) = appearance {
                    cm.insert_comp(*addr, ent, appearance.clone());
                }
                if let Some(animate) = animate {
                    cm.insert_comp(*addr, ent, animate.clone());
                }
                if tile.is_some() {
                    cm.insert_comp(*addr, ent, comn::art::Tile);
                }
            }
        }
    }
}

pub struct SpawnNewPlayers;
impl<'a> System<'a> for SpawnNewPlayers {
    type SystemData = (
        Entities<'a>,
        Read<'a, ConnectionManager>,
        Read<'a, LazyUpdate>,
        WriteStorage<'a, comn::net::SpawnPlayer>,
        ReadStorage<'a, Client>,
    );

    fn run(&mut self, (ents, cm, lu, mut players_to_spawn, clients): Self::SystemData) {
        use comn::{
            art::{Animate, Appearance},
            Iso2, Vec2,
        };
        for (_, ent, Client(new_player_addr)) in (players_to_spawn.drain(), &*ents, &clients).join()
        {
            trace!("spawning new player!");
            // these are the components the entity will have.
            let appearance = Appearance::GleamyStalagmite;
            let iso = Pos(Iso2::new(Vec2::new(1.0, 1.0), na::zero()));
            let animate = Animate::new();

            // give them player components
            lu.insert(ent, iso.clone());
            lu.insert(ent, appearance.clone());
            lu.insert(ent, animate.clone());

            // tell everyone 'bout the new kid on the block
            for Client(addr) in (&clients).join() {
                cm.new_ent(*addr, ent);
                cm.insert_comp(*addr, ent, iso.clone());
                cm.insert_comp(*addr, ent, appearance.clone());
                cm.insert_comp(*addr, ent, animate.clone());
                if addr == new_player_addr {
                    debug!("so we did tell them about themself");
                }
            }
        }
    }
}
