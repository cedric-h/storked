mod connection_manager;
mod login;
mod packets;
mod phys;

// main.rs needs to put these Systems in the graph
pub use login::SendWorldToNewPlayers;
pub use login::SpawnNewPlayers;
pub use packets::HandleClientPackets;
pub use phys::SendNewPositions;

// next we define a few components we'll need to do networking.
use comn::specs::prelude::*;
use specs_derive::Component;

#[derive(Component, Clone, Debug, Default)]
#[storage(NullStorage)]
pub struct LoggingIn;

#[derive(Component, Clone, Debug)]
#[storage(DenseVecStorage)]
pub struct Client(std::net::SocketAddr);

// the submodules can use this to gain access to structs they all need.
pub mod prelude {
    pub use super::{connection_manager::ConnectionManager, Client, LoggingIn};
}
