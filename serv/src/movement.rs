/*
use crate::net::prelude::*;
use comn::{
    art::{player_anim::PlayerAnimation, Animate, PlayerAnimationController},
    controls::Heading,
    prelude::*,
    specs,
};
use log::*;
use specs::prelude::*;

pub struct MoveHeadings;
impl<'a> System<'a> for MoveHeadings {
    type SystemData = (
        Entities<'a>,
        Read<'a, ConnectionManager>,
        ReadStorage<'a, Client>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Heading>,
        WriteStorage<'a, Animate>,
        ReadStorage<'a, PlayerAnimationController>,
    );

    fn run(
        &mut self,
        (ents, cm, clients, mut isos, mut heads, mut animates, anim_controls): Self::SystemData,
    ) {
        for (ent, iso, &mut Heading { mut dir }, player_anim_control, animaybe) in (
            &*ents,
            &mut isos,
            &mut heads,
            anim_controls.maybe(),
            (&mut animates).maybe(),
        )
            .join()
        {
            if dir.magnitude() > 0.0 {
                dir.renormalize();
                iso.0.translation.vector += dir.into_inner() * 0.7;

                if let (true, Some(anim)) = (player_anim_control.is_some(), animaybe) {
                    use comn::art::player_anim::Direction::*;

                    let direction = if dir.x > 0.0 {
                        Right
                    } else if dir.x < 0.0 {
                        Left
                    } else if dir.y > 0.0 {
                        Down
                    } else {
                        Up
                    };

                    let row = PlayerAnimation::Walk(direction).into();
                    anim.row = row;
                    for Client(addr) in (&clients).join() {
                        cm.insert_comp(*addr, ent, anim.clone());
                    }
                }
            }
        }
    }
}*/
