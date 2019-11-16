#![feature(stmt_expr_attributes)]
use comn::{
    prelude::*,
    specs::{self, prelude::*},
};
use log::*;
use specs::WorldExt;

mod movement;
mod net;

fn main() {
    {
        use log::LevelFilter::*;

        #[rustfmt::skip]
        pretty_env_logger::formatted_builder()
            .filter(None,                   Debug)
            .init();
    }

    let mut world = specs::World::new();
    #[rustfmt::skip]
    let mut dispatcher = DispatcherBuilder::new()
        .with(movement::MoveHeadings,       "heading",          &[])
        .with(net::SendWorldToNewPlayers,   "send world",       &[])
        .with(net::HandleClientPackets,     "client packets",   &["send world"])
        .with(net::SpawnNewPlayers,         "new players",      &["client packets"])
        .with(comn::dead::ClearDead,        "clear dead",       &["client packets"])
        .with(net::SendNewPositions,        "send pos",         &["clear dead"])
        .build();

    dispatcher.setup(&mut world);

    use comn::art::{Appearance, Tile};
    for x in 0..10 {
        for y in 0..10 {
            world
                .create_entity()
                .with(Tile)
                .with(if x * y % 3 == 0 {
                    if rand::random() {
                        Appearance::Rock
                    } else {
                        Appearance::SpottedRock
                    }
                } else {
                    Appearance::RockHole
                })
                .with(Pos(Iso2::translation(
                    x as f32 * 2.0 + 5.0,
                    y as f32 * 2.0 + 5.0,
                )))
                .build();
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
