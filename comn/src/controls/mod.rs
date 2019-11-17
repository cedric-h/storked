use super::{na, Vec2};
use serde::{Deserialize, Serialize};
use specs::{prelude::*, Component};

pub mod movement;
pub use movement::MoveHeadings;

#[derive(Clone, Debug, Component, Serialize, Deserialize)]
/// Nobody gets these on the Server, but the Server
/// will tell the Client to put one on the entity the Client
/// is looking out of at the moment.
/// NOTE: This isn't used atm.
pub struct Camera;

#[derive(Clone, Debug, Component, Serialize, Deserialize)]
/// Where would the Client like to go?
/// Note that the server isn't necessarily going to actually get them there.
pub struct Heading {
    pub dir: na::Unit<Vec2>,
}
