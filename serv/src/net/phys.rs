// our code
use super::prelude::*;
//use log::*;
// crates
use comn::{specs::prelude::*, Pos};

/// This system sends the world to all clients with the LoggingIn component.
pub struct SendNewPositions;
impl<'a> System<'a> for SendNewPositions {
    type SystemData = (
        // things we need to do networking
        Read<'a, ConnectionManager>,
        ReadStorage<'a, LoggingIn>,
        ReadStorage<'a, Client>,
        // things we need to tell new players about
        Entities<'a>,
        ReadStorage<'a, Pos>,
    );

    fn run(&mut self, (cm, loggin_ins, clients, ents, isos): Self::SystemData) {
        for (Client(addr), _) in (&clients, !&loggin_ins).join() {
            for (Pos(iso), ent) in (&isos, &*ents).join() {
                use std::time::{SystemTime, UNIX_EPOCH};
                cm.insert_comp(
                    *addr,
                    ent,
                    comn::net::UpdatePosition {
                        iso: iso.clone(),
                        time_stamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap(),
                    },
                );
            }
        }
    }
}
