pub use enum_iterator;
pub use nalgebra as na;
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

#[derive(Clone, Debug, Component, Serialize, Deserialize)]
pub struct Pos(pub Iso2);

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
        use super::{SpawnPlayer, UpdatePosition};
        use crate::art::{Animate, Appearance, Tile};
        use crate::controls::{Camera, Heading};
        use crate::dead::Dead;

        net_component! {
            Appearance,
            Tile,
            Animate,
            Pos,
            Dead,
            UpdatePosition,
            SpawnPlayer,
            Heading,
            Camera,
        }
    }
}
pub use net::{NetComponent, NetMessage};

pub mod art {
    use enum_iterator::IntoEnumIterator;
    use serde::{Deserialize, Serialize};
    use specs::{prelude::*, Component};

    #[derive(Clone, Debug, Default, Component, Serialize, Deserialize)]
    /// Entities with this component are rendered at a special stage on the client,
    /// and their origin is in the (center, center) rather than their (center, bottom)
    #[storage(NullStorage)]
    pub struct Tile;

    #[derive(Clone, Debug, Default, Component, Serialize, Deserialize)]
    /// Entities with this component are rendered at a special stage on the client,
    /// and their origin is in the (center, center) rather than their (center, bottom)
    pub struct Animate {
        current_frame: usize,
    }

    impl Animate {
        pub fn new() -> Self {
            Self { current_frame: 0 }
        }
    }

    #[derive(Clone)]
    pub struct AnimationData {
        total_frames: usize,
        /// How long to spend on one frame.
        frame_duration: usize,
        frame_width: usize,
        frame_height: usize,
    }

    #[derive(
        PartialEq, Eq, Hash, Clone, Debug, IntoEnumIterator, Component, Serialize, Deserialize,
    )]
    /// Behavior can affect how something is rendered on the client, but
    /// the appearance should never affect the behavior.
    /// Therefore, this component isn't really used on the server all that much
    /// except for when it needs to be sent down to the clients.
    pub enum Appearance {
        Rock,
        SpottedRock,
        RockHole,
        GleamyStalagmite,
    }
    impl Appearance {
        pub fn animation_data() -> std::collections::HashMap<Appearance, AnimationData> {
            [(
                Appearance::GleamyStalagmite,
                AnimationData {
                    total_frames: 4,
                    frame_duration: 5,
                    frame_width: 32,
                    frame_height: 32,
                },
            )]
            .iter()
            .cloned()
            .collect()
        }
    }
}

pub mod dead {
    use serde::{Deserialize, Serialize};
    use specs::{prelude::*, Component};

    #[derive(Clone, Default, Debug, Component, Serialize, Deserialize)]
    #[storage(NullStorage)]
    /// Marking an entity with this component means that it will be cleared away
    /// before the next update. A component which performs this function is more
    /// convenient than the simple Entities.delete() method because it allows
    /// neat cleanup to be done before the final removal of the entity.
    pub struct Dead;

    /// This system clears away the entities that have died and are no longer needed.
    pub struct ClearDead;
    impl<'a> System<'a> for ClearDead {
        type SystemData = (Entities<'a>, WriteStorage<'a, Dead>);

        fn run(&mut self, (ents, mut dead): Self::SystemData) {
            for (ent, _) in (&*ents, dead.drain()).join() {
                ents.delete(ent)
                    .expect("Couldn't kill entity with Dead component");
            }
        }
    }
}
pub use dead::Dead;

pub mod controls {
    use super::{na, Vec2};
    use serde::{Deserialize, Serialize};
    use specs::{prelude::*, Component};

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
}
