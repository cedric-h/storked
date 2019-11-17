use crate::prelude::*;
use crate::{collide, controls::Heading, Cuboid, Fps, Hitbox};
use specs::prelude::*;

/// Currently, Collision serves to prevent people who are trying to go through things
/// from going through those things.
pub struct Collision;
impl<'a> System<'a> for Collision {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Pos>,
        ReadStorage<'a, Hitbox>,
        ReadStorage<'a, Heading>,
    );

    fn run(&mut self, (ents, mut poses, hitboxes, headings): Self::SystemData) {
        use collide::query::contact;
        use na::Translation2;

        // for everyone going somewhere...
        (&*ents, &poses, &hitboxes, &headings)
            .join()
            .filter_map(|(ent, Pos(iso), Hitbox(hb), _)| {
                // for everything they could collide with...
                for (o_ent, Pos(o_iso), Hitbox(o_hb)) in (&*ents, &poses, &hitboxes).join() {
                    if ent != o_ent {
                        // they're touching the goer! goer goes back!
                        // The Translation sorcery moves the hitbox up by the height of
                        // the box, because that's how entities are rendered.
                        if let Some(c) = contact(
                            &(iso * Translation2::from(Vec2::y() * -2.0 * hb.half_extents().y)),
                            hb,
                            &(o_iso * Translation2::from(Vec2::y() * -2.0 * o_hb.half_extents().y)),
                            o_hb,
                            0.0,
                        ) {
                            return Some((ent, c.normal.into_inner() * c.depth));
                        }
                    }
                }
                None
            })
            .collect::<Vec<_>>()
            .iter()
            .for_each(|(ent, normal)| {
                let pos = poses.get_mut(*ent).unwrap();
                pos.0.translation.vector -= normal;
            });
    }
}
