//! These structs make updating the player's Animate
//! to reflect its Heading easier.

#[derive(Copy, Clone)]
#[cfg_attr(test, derive(PartialEq))]
pub enum Direction {
    Up,
    Left,
    Down,
    Right,
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Copy, Clone)]
pub enum PlayerAnimation {
    Cast(Direction),
    Jab(Direction),
    Walk(Direction),
    Swing(Direction),
    Shoot(Direction),
    Die,
}
impl Into<usize> for PlayerAnimation {
    fn into(self) -> usize {
        use PlayerAnimation::*;

        match self {
            Cast(d) => d as usize,
            Jab(d) => 4 + d as usize,
            Walk(d) => 8 + d as usize,
            Swing(d) => 12 + d as usize,
            Shoot(d) => 16 + d as usize,
            Die => 20,
        }
    }
}
#[test]
fn test_player_anim_indexing() {
    pub use Direction::*;
    pub use PlayerAnimation::*;
    assert_eq!(
        #[rustfmt::skip]
        [
            Cast(Up),   Cast(Left),     Cast(Down),     Cast(Right),
            Jab(Up),    Jab(Left),      Jab(Down),      Jab(Right),
            Walk(Up),   Walk(Left),     Walk(Down),     Walk(Right),
            Swing(Up),  Swing(Left),    Swing(Down),    Swing(Right),
            Shoot(Up),  Shoot(Left),    Shoot(Down),    Shoot(Right),
            Die,
        ]
        .iter()
        .map(|anim: &PlayerAnimation| -> usize { (*anim).into() })
        .collect::<Vec<usize>>(),
        (0..=20).collect::<Vec<usize>>(),
    )
}

use serde::{Deserialize, Serialize};
use specs::{prelude::*, Component};
#[derive(Clone, Debug, Component, Serialize, Deserialize)]
pub struct PlayerAnimationController;
