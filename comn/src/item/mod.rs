use crate::prelude::*;
use serde::{Deserialize, Serialize};
use specs::{prelude::*, Component};

#[derive(Clone, Debug, Default, Component, Serialize, Deserialize)]
/// Something that can be put inside of an inventory.
pub struct Item;

#[derive(Clone, Debug, Default, Component, Serialize, Deserialize)]
pub struct PickupRequest {
    /// The id of the Entity the Player would like to put in their inventory.
    pub id: u32,
}

#[derive(Clone, Debug, Default, Component, Serialize, Deserialize)]
pub struct DropRequest {
    /// The id of the Entity the Player would like to throw on the ground.
    pub id: u32,
}

#[derive(Clone, Debug, Default, Component, Serialize, Deserialize)]
/// This Component stores the server ids of all of the items which are owned by
/// the entity with which this Component is associated.
pub struct Inventory {
    pub items: Vec<u32>,
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
