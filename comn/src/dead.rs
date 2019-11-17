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
