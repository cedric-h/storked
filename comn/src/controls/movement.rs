use crate::{
    art::{player_anim::PlayerAnimation, Animate, PlayerAnimationController},
    controls::Heading,
    prelude::*,
    Fps,
};
use specs::prelude::*;

pub struct MoveHeadings;
impl<'a> System<'a> for MoveHeadings {
    type SystemData = (
        Read<'a, Fps>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Heading>,
        WriteStorage<'a, Animate>,
        ReadStorage<'a, PlayerAnimationController>,
    );

    fn run(&mut self, (fps, mut isos, mut heads, mut animates, anim_controls): Self::SystemData) {
        for (iso, &mut Heading { mut dir }, player_anim_control, animaybe) in (
            &mut isos,
            &mut heads,
            anim_controls.maybe(),
            (&mut animates).maybe(),
        )
            .join()
        {
            if dir.magnitude() > 0.0 {
                dir.renormalize();

                // 20 fps = 3, 60 fps = 1
                let update_granularity = 1.0 / fps.0 * 60.0;
                iso.0.translation.vector += dir.into_inner() * 0.135 * update_granularity;

                if let (true, Some(anim)) = (player_anim_control.is_some(), animaybe) {
                    use crate::art::player_anim::Direction::*;

                    let direction = if dir.x > 0.0 {
                        Right
                    } else if dir.x < 0.0 {
                        Left
                    } else if dir.y > 0.0 {
                        Down
                    } else {
                        Up
                    };

                    anim.row = PlayerAnimation::Walk(direction).into();
                }
            } else {
                if let (true, Some(anim)) = (player_anim_control.is_some(), animaybe) {
                    anim.current_frame = 0;
                }
            }
        }
    }
}
