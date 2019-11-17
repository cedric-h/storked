use enum_iterator::IntoEnumIterator;
use serde::{Deserialize, Serialize};
use specs::{prelude::*, Component};

pub mod player_anim;
pub use player_anim::PlayerAnimationController;

#[derive(Clone, Debug, Default, Component, Serialize, Deserialize)]
/// Entities with this component are rendered at a special stage on the client,
/// and their origin is in the (center, center) rather than their (center, bottom)
#[storage(NullStorage)]
pub struct Tile;

#[derive(Clone, Debug, Default, Component, Serialize, Deserialize)]
/// Entities with this component are rendered at a special stage on the client,
/// and their origin is in the (center, center) rather than their (center, bottom)
pub struct Animate {
    pub current_frame: usize,
    pub row: usize,
}

impl Animate {
    pub fn new() -> Self {
        Self {
            current_frame: 0,
            row: 0,
        }
    }
    pub fn row(row: usize) -> Self {
        Self {
            current_frame: 0,
            row,
        }
    }
}

pub struct UpdateAnimations;
impl<'a> System<'a> for UpdateAnimations {
    type SystemData = (WriteStorage<'a, Animate>, ReadStorage<'a, Appearance>);

    fn run(&mut self, (mut animates, appearances): Self::SystemData) {
        for (animate, appearance) in (&mut animates, &appearances).join() {
            let SpritesheetData { rows, .. } = crate::art::SPRITESHEETS
                .get(appearance)
                .unwrap_or_else(|| panic!("No animation data found for {:?}!", appearance));

            let AnimationData {
                total_frames,
                frame_duration,
            } = rows
                .get(animate.row)
                .unwrap_or_else(|| panic!("{:?} has no row #{}!", appearance, animate.row));

            animate.current_frame += 1;

            // greater than or equal to because it starts at 0
            if animate.current_frame >= total_frames * frame_duration {
                animate.current_frame = 0;
            }
        }
    }
}

#[derive(Clone)]
/// An animation is stored on one row of a spritesheet.
pub struct AnimationData {
    pub total_frames: usize,
    /// How long to spend on one frame.
    pub frame_duration: usize,
}

#[derive(Clone)]
/// A spritesheet stores several animations in rows.
/// Each column is a new frame in each animation.
/// Every frame has the same height and width.
pub struct SpritesheetData {
    pub rows: Vec<AnimationData>,
    pub frame_width: usize,
    pub frame_height: usize,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug, IntoEnumIterator, Component, Serialize, Deserialize)]
/// Behavior can affect how something is rendered on the client, but
/// the appearance should never affect the behavior.
/// Therefore, this component isn't really used on the server all that much
/// except for when it needs to be sent down to the clients.
pub enum Appearance {
    Rock,
    SpottedRock,
    RockHole,
    GleamyStalagmite,
    Player,
}

lazy_static::lazy_static! {
    pub static ref SPRITESHEETS: std::collections::HashMap<Appearance, SpritesheetData> = {
        use Appearance::*;
        [
            (
                GleamyStalagmite,
                SpritesheetData {
                    rows: vec![AnimationData {
                        total_frames: 4,
                        frame_duration: 12,
                    }],
                    frame_width: 32,
                    frame_height: 32,
                },
            ),
            (
                Player,
                SpritesheetData {
                    rows: {
                        let mut rows = [
                            // (total frames, frame duration)
                            (7,     12),    // Casting
                            (8,     12),    // Jabbing
                            (9,     6),     // Walking
                            (6,     12),    // Swinging
                            (13,    12),    // Shooting
                        ]
                        .iter()
                        .fold(
                            Vec::new(),
                            // There are actually four rows for each of casting, jabbing etc.
                            |mut rows, &(total_frames, frame_duration)| {
                                for _ in 0..4 {
                                    rows.push(AnimationData {total_frames, frame_duration});
                                }
                                rows
                            },
                        );

                        // Dying
                        rows.push(AnimationData {
                            total_frames: 6,
                            frame_duration: 12,
                        });

                        rows
                    },
                    frame_width: 64,
                    frame_height: 64,
                },
            ),
        ]
        .iter()
        .cloned()
        .collect()
    };
}
