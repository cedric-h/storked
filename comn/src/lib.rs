#![feature(stmt_expr_attributes)]

pub use enum_iterator;
pub use nalgebra as na;
pub use ncollide2d as collide;
pub use rmp_serde as rmps;
pub use serde;
pub use specs;

pub mod prelude {
    pub use super::na;
    pub use super::specs;
    pub use super::{Iso2, Pos, Vec2};
}
use prelude::*;
use serde::{Deserialize, Serialize};
use specs::{prelude::*, Component};

pub type Vec2 = na::Vector2<f32>;
pub type Iso2 = na::Isometry2<f32>;
pub use collide::shape::Cuboid;

#[derive(Clone, Debug, Component, Serialize, Deserialize)]
pub struct Pos(pub Iso2);

impl Pos {
    pub fn vec(vec: Vec2) -> Self {
        Pos(Iso2::new(vec, na::zero()))
    }
}

#[derive(Clone, Debug, Component, Serialize, Deserialize)]
pub struct Hitbox(pub Cuboid<f32>);

#[derive(Default)]
pub struct Fps(pub f32);

pub mod art;

pub mod dead;
pub use dead::Dead;

pub mod controls;

pub mod phys;

pub mod net {
    pub use comp::NetComponent;
    pub use msg::NetMessage;
    // UpdatePosition
    use super::prelude::*;
    use serde::{Deserialize, Serialize};
    use specs::{prelude::*, Component};

    #[derive(Clone, Debug, Component, Serialize, Deserialize)]
    /// These wrap around an Iso2.
    /// They're sent from the Server to the Client
    /// to update positions, no entity on the Server
    /// should have one of those, though they should
    /// be fairly common on the Client.
    pub struct UpdatePosition {
        pub iso: Iso2,
        // duration since UNIX_EPOCH
        pub time_stamp: std::time::Duration,
    }

    #[derive(Clone, Debug, Component, Serialize, Deserialize)]
    /// This is sent in by the player when they're ready
    /// for their Pos and Appearance components.
    /// Essentially, when they want to enter the game world.
    /// Menu/Spectator -> Game
    pub struct SpawnPlayer;

    #[derive(Clone, Debug, Component, Serialize, Deserialize)]
    /// The server attaches this to an entity on the clients to
    /// tell clients which entity they are able to control.
    pub struct LocalPlayer;

    mod msg {
        use super::NetComponent;
        use serde::{Deserialize, Serialize};

        #[derive(Deserialize, Serialize, Debug)]
        pub enum NetMessage {
            NewEnt(u32),
            InsertComp(u32, NetComponent),
        }
    }

    mod comp {
        // util includes
        use crate::Pos;
        use serde::{Deserialize, Serialize};
        use specs::{Entity, LazyUpdate};

        macro_rules! net_component_base {
            ( $( $x:tt : $y:ty $(: $extra:ident)? ),+ $(,)? ) => {
                #[derive(Deserialize, Serialize, Debug)]
                pub enum NetComponent {
                    $(
                        $x($y),
                    )+
                }

                $(
                    impl From<$y> for NetComponent {
                        fn from(c: $y) -> Self {
                            NetComponent::$x(c)
                        }
                    }
                )+

                impl NetComponent {
                    pub fn insert(self, ent: Entity, lu: &LazyUpdate) {
                        match self {
                            $(
                                NetComponent::$x(c) => lu.insert(ent, c),
                            )+
                        }
                    }
                }
            };
        }

        macro_rules! net_component {
            ( $( $name:ident $(: $inner:ty)? ),+ $(,)? ) => {
                net_component_base! {
                    $($name $(: $inner)? : $name),*
                }
            }
        }

        // Component includes
        use super::{LocalPlayer, SpawnPlayer, UpdatePosition};
        use crate::art::{Animate, Appearance, PlayerAnimationController, Tile};
        use crate::controls::{Camera, Heading};
        use crate::dead::Dead;
        use crate::Hitbox;

        net_component! {
            Appearance,
            Tile,
            Animate,
            PlayerAnimationController,
            Pos,
            Hitbox,
            Dead,
            UpdatePosition,
            SpawnPlayer,
            LocalPlayer,
            Heading,
            Camera,
        }
    }
}
pub use net::{NetComponent, NetMessage};
