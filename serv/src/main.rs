#![feature(stmt_expr_attributes)]
use comn::{
    prelude::*,
    specs::{self, prelude::*},
};
use log::*;
use specs::WorldExt;
mod net;
mod pickup;

fn main() {
    {
        use log::LevelFilter::*;

        #[rustfmt::skip]
        pretty_env_logger::formatted_builder()
            .filter(None,                   Debug)
            .init();
    }

    let mut world = specs::World::new();
    world.insert(comn::Fps(20.0));
    #[rustfmt::skip]
    let mut dispatcher = DispatcherBuilder::new()
        .with(pickup::ItemPickupDrop,       "pickup",           &[])
        .with(comn::art::UpdateAnimations,  "animate",          &[])
        .with(comn::phys::Collision,        "collision",        &[])
        .with(comn::controls::MoveHeadings, "heading",          &[])
        .with(net::SendWorldToNewPlayers,   "send world",       &[])
        .with(net::HandleClientPackets,     "client packets",   &["send world"])
        .with(net::SpawnNewPlayers,         "new players",      &["client packets"])
        .with(comn::dead::ClearDead,        "clear dead",       &["client packets"])
        .with(net::SendNewPositions,        "send pos",         &["clear dead"])
        .build();

    dispatcher.setup(&mut world);

    use comn::art::{Animate, Appearance, Tile};
    use comn::{Cuboid, Hitbox};
    use rand::{thread_rng, Rng};
    let mut rng = thread_rng();
    for x in 0..10 {
        for y in 0..10 {
            let is_hole = x * y % 3 != 0;
            let loc = Vec2::new(x as f32 * 2.0 + 2.0, y as f32 * 2.0 + 2.0);

            world
                .create_entity()
                .with(Tile)
                .with({
                    use Appearance::*;

                    if is_hole {
                        RockHole
                    } else if rand::random() {
                        Rock
                    } else {
                        SpottedRock
                    }
                })
                .with(Pos::vec(loc.clone()))
                .build();

            match (is_hole, rng.gen_range(0, 10)) {
                (true, 4) => {
                    world
                        .create_entity()
                        .with(Appearance::GleamyStalagmite)
                        .with(Pos::vec(loc + Vec2::y() * 0.75))
                        .with(Hitbox(Cuboid::new(Vec2::new(0.8, 0.5))))
                        .with(Animate::new())
                        .build();
                }
                (false, 3) => {
                    if rand::random() {
                        world
                            .create_entity()
                            .with(Item)
                            .with(Appearance::Key)
                            .with(Pos::vec(loc + Vec2::y() * 0.75))
                            .build();
                    }
                }
                _ => {}
            }
        }
    }

    info!("starting game loop!");

    let mut fixedstep = fixedstep::FixedStep::start(20.0); // 20.0Hz

    loop {
        while fixedstep.update() {
            dispatcher.dispatch(&mut world);
            world.maintain();
        }
    }
}
