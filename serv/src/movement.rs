use comn::{controls::Heading, prelude::*, specs};
use log::*;
use specs::prelude::*;

pub struct MoveHeadings;
impl<'a> System<'a> for MoveHeadings {
    type SystemData = (WriteStorage<'a, Pos>, WriteStorage<'a, Heading>);

    fn run(&mut self, (mut isos, mut heads): Self::SystemData) {
        for (iso, &mut Heading { mut dir }) in (&mut isos, &mut heads).join() {
            if dir.magnitude() > 0.0 {
                dir.renormalize();
                iso.0.translation.vector += dir.into_inner();
            }
        }
    }
}
