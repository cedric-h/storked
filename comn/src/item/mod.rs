use crate::prelude::*;
use serde::{Deserialize, Serialize};
use specs::{prelude::*, Component};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Component, Serialize, Deserialize)]
/// Something that can be put inside of an inventory.
pub enum Item {
    /// An Item of this variant should also have a Weapon component.
    Weapon,

    /// Just your normal everyday item.
    /// These can only be stored in Slot::Loose, whereas the
    /// other things can also fit in Slot::Reserved for their particular
    /// type of Item.
    Misc,
}
impl Default for Item {
    fn default() -> Self {
        Item::Misc
    }
}

#[derive(Clone, Debug, Default, Component, Serialize, Deserialize)]
pub struct PickupRequest {
    /// The id of the Entity the Player would like to put in their inventory.
    pub id: u32,
}

#[derive(Clone, Debug, Default, Component, Serialize, Deserialize)]
/// Note that unlike PickupRequest, DropRequest refers to an item based on the given
/// item's location in the player's inventory, not that item's index in the ECS.
///
/// There are a couple of reasons for this. First of all, this acts as a security measure;
/// if raw ids were accepted here, players could potentially drop items that are actually
/// in other people's inventories.
///
/// Such a vulnerability could be avoided with a couple of preemptive checks,
/// but in this case, a DropRequest is actually also the more performant option,
/// since Inventories are indexed into by SlotIndexes.
///
/// Furthermore, being able to drop an item based on a SlotIndex is easier to accomodate on
/// the client, where the GUI widgets are already arranged based on the data in their SlotIndexes.
///
/// This does have a couple of drawbacks, however: there are certain situations where it would
/// actually be beneficial for the player to be able to drop an item from an inventory that isn't
/// their own, as the case may be with, for example, chests that they have open.
///
/// That bridge will be crossed when it is come to.
pub struct DropRequest {
    /// The SlotIndex of the item the player would like to throw on the ground.
    pub item_index: SlotIndex,
}

#[derive(Debug)]
pub enum Error {
    InvalidSlotIndex,
    InventoryFull,
}

#[derive(Clone, Debug, Default, Component, Serialize, Deserialize)]
/// #Overview
/// This Component stores the server ids of all of the items which are owned by
/// the entity with which this Component is associated.
/// Individual items are referred to using Slots.
/// Inside of a Slot, the u32 of an entity may or may not be stored.
///
/// #Anatomy of an Inventory
/// Conceptually, an inventory is comprised of two parts:
/// - Reserved Slots: The slots reserved for a certain purpose, i.e. a player's 'Sword' slot,
/// in which only a weapon can exist.
/// - Loose Inventory: The slots which are reserved for no purpose in particular,
/// in which any variety of item can exist.
///
/// This distinction becomes important when, for example, inserting items.
/// Items should be inserted linearly starting from the top left into the loose inventory,
/// unless a reserved slot which accepts an item of the variety being inserted exists and is empty,
/// in which case the reserved slot should take priority.
pub struct Inventory {
    // The internal representation of the Inventory
    items: HashMap<SlotIndex, Option<u32>>,
    /// The number of rows of Loose Inventory available.
    rows: usize,
    /// The number of columns of Loose Inventory available.
    columns: usize,
}

impl Inventory {
    #[inline]
    /// A `character` inventory has Reserved Slots for gear,
    /// as well as some amount of Loose Inventory.
    pub fn character() -> Self {
        let mut inv = Self::new_loose(6, 2);
        inv.items.insert(SlotIndex::Reserved(Item::Weapon), None);
        inv
    }

    #[inline]
    /// Create an Inventory comprised of only Loose Inventory with the given dimensions.
    pub fn new_loose(rows: usize, cols: usize) -> Self {
        let mut items = HashMap::new();

        for row in 0..rows {
            for col in 0..cols {
                items.insert(SlotIndex::Loose(row, col), None);
            }
        }

        Inventory {
            items,
            rows,
            columns: cols,
        }
    }

    #[inline]
    /// Returns an iterator over the Loose Inventory
    pub fn loose(&self) -> impl Iterator<Item = (&SlotIndex, &Option<u32>)> {
        self.items.iter().filter(|(i, _)| i.is_loose())
    }

    #[inline]
    /// Returns an iterator over the Reserved Slots
    pub fn reserved(&self) -> impl Iterator<Item = (&SlotIndex, &Option<u32>)> {
        self.items.iter().filter(|(i, _)| i.is_reserved())
    }

    #[inline]
    /// Returns a reference to the slot at the given SlotIndex
    /// if such a slot exists. Otherwise, an error is returned.
    pub fn slot(&self, index: &SlotIndex) -> Result<&Option<u32>, Error> {
        self.items
            .get(index)
            .map(|i| Ok(i))
            .unwrap_or_else(|| Err(Error::InvalidSlotIndex))
    }

    #[inline]
    /// Clears the slot at the given SlotIndex by setting it to None,
    /// and returns the value of the slot previous to clearing it.
    /// Returns an error if no slot with that index cannot be found.
    pub fn clear(&mut self, index: &SlotIndex) -> Result<Option<u32>, Error> {
        match self.items.insert(index.clone(), None) {
            // if this slot existed beforehand as it should have, return that
            Some(existing) => Ok(existing),
            // if it didn't, that means the slot at this index was never made and
            // that's terrifying.
            None => {
                // remove the item that was just inserted,
                // this inventory wasn't supposed to have a slot like that.
                self.items.remove(index.clone());

                Err(Error::InvalidSlotIndex)
            },
        }
    }

    #[inline]
    /// Finds an empty slot in the Loose Inventory, and inserts the provided item entity id into it.
    ///
    /// If an empty slot can be found, its index returned,
    /// but if no empty slot can be found an error is returned.
    pub fn insert_loose(&mut self, ent: u32) -> Result<SlotIndex, Error> {
        for col in 0..self.columns {
            for row in 0..self.rows {
                let index = SlotIndex::Loose(row, col);
                if self
                    .slot(&index)
                    .unwrap_or_else(|_| {
                        panic!(
                            "Inventory improperly formed;\
                             no SlotIndex::Loose at {:?} which is within inventory's size of {:?}",
                            (row, col),
                            (self.rows, self.columns),
                        )
                    })
                    .is_none()
                {
                    self.items.insert(index.clone(), Some(ent));
                    return Ok(index);
                }
            }
        }
        Err(Error::InventoryFull)
    }

    #[inline]
    /// Finds an empty slot appropriate for the Entity provided, and inserts the entity into it.
    ///
    /// Reserved Slots are prioritized over Loose Inventory, and Loose Inventory is filled starting
    /// from the top left.
    ///
    /// An Item component is required so that it can be determined if the provided Entity is
    /// eligble for positioning inside of any Reserved Slot.
    ///
    /// If an empty slot can be found, a tuple representing its (row, column) is returned,
    /// but if no empty slot can be found an error is returned.
    pub fn insert(&mut self, ent: u32, item: &Item) -> Result<SlotIndex, Error> {
        // first try to get that reserved spot for this item
        if Item::Misc != *item {
            let index = SlotIndex::Reserved(item.clone());
            if let Some(slot) = self.items.get_mut(&index) {
                // make sure the slot's empty
                if slot.is_none() {
                    *slot = Some(ent);
                    return Ok(index);
                }
            }
        }

        // if that didn't work out for any of a number of reasons, try loose
        // (which will return an error if it can't find anything)
        self.insert_loose(ent)
    }
}

/// A SlotIndex refers to a particular place in an Inventory.
///
/// See the documentation on Inventory for a better understanding of what places exist in an
/// Inventory.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum SlotIndex {
    /// These slots can only store items of certain kinds.
    /// There can also only be one of this kind of slot for any given item
    /// in any given Inventory.
    Reserved(Item),
    /// Whereas the other variants of Slot are dedicated to items of
    /// a certain nature, Loose Slots can contain any sort of item.
    /// (row, column)
    Loose(usize, usize),
}
impl Default for SlotIndex {
    fn default() -> Self {
        Self::Loose(0, 0)
    }
}
impl SlotIndex {
    pub fn is_reserved(&self) -> bool {
        match self {
            SlotIndex::Reserved(_) => true,
            _ => false,
        }
    }
    pub fn is_loose(&self) -> bool {
        match self {
            SlotIndex::Loose(_, _) => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, Default, Component, Serialize, Deserialize)]
/// This Component removes the Pos Component from an entity, preventing it
/// from interacting physically from the world, making it able to be used as
/// an item.
pub struct Deposition;

pub const MAX_INTERACTION_DISTANCE_SQUARED: f32 = {
    let f = 2.0;
    f * f
};
