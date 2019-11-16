#![recursion_limit = "256"]
#[macro_use]
extern crate stdweb;
use stdweb::web::window;

pub mod prelude {
    pub use comn::prelude::*;
    pub use comn::rmps;
    pub use specs::{prelude::*, Component};
}
use prelude::*;

mod renderer {
    use crate::prelude::*;
    use comn::art::{Animate, AnimationData, Appearance, Tile};
    use comn::enum_iterator::IntoEnumIterator;
    use std::collections::HashMap;
    use stdweb::{
        traits::*,
        unstable::TryInto,
        web::{
            html_element::{CanvasElement, ImageElement},
            CanvasRenderingContext2d as CanvasContext, RenderingContext,
        },
    };

    pub struct Render {
        ctx: CanvasContext,
        imgs: HashMap<Appearance, ImageElement>,
        animation_data: HashMap<Appearance, AnimationData>,
    }

    impl Default for Render {
        fn default() -> Self {
            // find the thing we'll draw on
            let canvas: CanvasElement = stdweb::web::document()
                .get_element_by_id("canv")
                .expect("Couldn't find canvas to render on.")
                .try_into()
                .expect("Entity with the 'canv' id isn't a canvas!");

            // load up the images
            let imgs = Appearance::into_enum_iter()
                .map(|appearance| {
                    let loc = format!("./img/{:?}.png", appearance);

                    // set image up to load
                    let new_img = ImageElement::new();
                    new_img.set_src(&loc);

                    // log on image load
                    js!(@{new_img.clone()}.onload = () => console.log(@{loc}));

                    (appearance, new_img)
                })
                .collect();

            Self {
                ctx: CanvasContext::from_canvas(&canvas)
                    .expect("Couldn't get canvas rendering context from canvas"),
                imgs,
                animation_data: Appearance::animation_data(),
            }
        }
    }

    impl<'a> System<'a> for Render {
        type SystemData = (
            ReadStorage<'a, Appearance>,
            ReadStorage<'a, Pos>,
            ReadStorage<'a, Tile>,
            ReadStorage<'a, Animate>,
        );

        fn run(&mut self, (appearances, poses, tiles, animates): Self::SystemData) {
            self.ctx.set_fill_style_color("black");

            // black background
            let win = stdweb::web::window();
            self.ctx.fill_rect(
                0.0,
                0.0,
                win.inner_width().into(),
                win.inner_height().into(),
            );

            // tiles are rendered as if their origin was their center on the X and Y.
            // also, tiles are rendered first so that everything else can step on them.
            for (appearance, &Pos(iso), _) in (&appearances, &poses, &tiles).join() {
                const SIZE: f32 = 2.0;
                self.ctx
                    .draw_image_d(
                        self.imgs[appearance].clone(),
                        ((iso.translation.vector.x - SIZE / 2.0) * 20.0) as f64,
                        ((iso.translation.vector.y - SIZE / 2.0) * 20.0) as f64,
                        (SIZE * 20.0) as f64,
                        (SIZE * 20.0) as f64,
                    )
                    .expect("Couldn't draw tile!");
            }

            // other entities are rendered as if their origin was
            // their center on the X,
            // but their bottom on the Y.
            for (appearance, &Pos(iso), animaybe, _) in
                (&appearances, &poses, animates.maybe(), !&tiles).join()
            {
                const SIZE: f32 = 2.0;
                //if let Some(animate) = animaybe {
                //} else {
                self.ctx
                    .draw_image_d(
                        self.imgs[appearance].clone(),
                        ((iso.translation.vector.x - SIZE / 2.0) * 20.0) as f64,
                        ((iso.translation.vector.y - SIZE) * 20.0) as f64,
                        (SIZE * 20.0) as f64,
                        (SIZE * 20.0) as f64,
                    )
                    .expect("Couldn't draw non-tile entity!");
                // }
            }
        }
    }
}

mod net {
    use comn::{na, rmps, NetComponent, NetMessage, Pos};
    use specs::prelude::*;
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };
    use stdweb::{
        console,
        unstable::TryInto,
        web::{
            event::{SocketCloseEvent, SocketErrorEvent, SocketMessageEvent, SocketOpenEvent},
            ArrayBuffer, IEventTarget, WebSocket,
        },
        Value,
    };

    pub struct ServerConnection {
        ws: WebSocket,
        pub message_queue: Arc<Mutex<Vec<NetMessage>>>,
    }
    impl ServerConnection {
        #[inline]
        fn send(&self, msg: NetMessage) {
            self.ws
                .send_bytes(&rmps::encode::to_vec(&msg).expect("Couldn't encode NetMessage!"))
                .expect("Couldn't send NetMessage to server!");
        }

        /* I'm not sure why/when/how you'd ever even actually use this on the client.
         * The server should definitely be in control of when new things are made,
         * even if indirectly the Client ends up requesting that to happen.
         * For that reason, this is prevented from working on the serverside.
         * Instead, it's used internally to register a new player; if you send
         * this request through hacking or some other means, you'll just get
         * your player reset :grin:
        #[inline]
        pub fn new_ent(&self, ent: specs::Entity) {
            self.send(NetMessage::NewEnt(ent.id()));
        }*/

        #[inline]
        pub fn insert_comp<C: Into<NetComponent>>(
            &self,
            // The client can only request that components are
            // inserted onto itself.
            // ent: specs::Entity,
            comp: C,
        ) {
            // just using a 0 here for the entity ID since they can
            // only insert components onto their own entity.
            self.send(NetMessage::InsertComp(0, comp.into()));
        }
    }

    impl Default for ServerConnection {
        fn default() -> Self {
            let ws = WebSocket::new("ws://127.0.0.1:3012")
                .unwrap_or_else(|e| panic!("couldn't reach server: {}", e));
            let message_queue = Arc::new(Mutex::new(Vec::new()));

            ws.add_event_listener(|_: SocketOpenEvent| {
                console!(log, "Connected to server!");
            });

            ws.add_event_listener(|e: SocketErrorEvent| {
                js! {
                    console.error("Errror connecting to %s", @{e}.target.url);
                };
            });

            ws.add_event_listener(|e: SocketCloseEvent| {
                console!(error, "Server Connection Closed: %s", e.reason());
            });

            ws.add_event_listener({
                let msgs = message_queue.clone();

                move |msg: SocketMessageEvent| {
                    let msgs = msgs.clone();

                    let parse_msg_data = move |data: Value| {
                        let buf: ArrayBuffer = data
                            .try_into()
                            .expect("Couldn't turn server message into array buffer!");

                        let mut msgs = msgs.lock().expect("The Server Message Queue is locked!");
                        msgs.push(
                            rmps::from_read_ref::<Vec<u8>, _>(&buf.into())
                                .expect("couldn't read net message bytes"),
                        );
                    };

                    js! {
                        let reader = new FileReader();
                        reader.addEventListener("loadend", () => {
                            let parse = @{parse_msg_data};
                            parse(reader.result);
                            parse.drop();
                        });
                        reader.readAsArrayBuffer(@{msg}.data);
                    };
                }
            });

            Self { ws, message_queue }
        }
    }

    use comn::net::UpdatePosition;
    pub struct SyncPositions;
    impl<'a> System<'a> for SyncPositions {
        type SystemData = (WriteStorage<'a, Pos>, ReadStorage<'a, UpdatePosition>);

        // the idea here is to get wherever the client thinks something is to where the server has
        // it at within 10 ms.
        // You want to do that transition gradually to avoid sudden jerking.
        // If the internet is being slow and the update is from a while ago, however, it's probably
        // more apt to just rely on the physics simulation on the client than on the last position
        // the server sent; that way things in the simulation will still move.
        fn run(&mut self, (mut currents, updates): Self::SystemData) {
            for (Pos(current), UpdatePosition { iso: update, .. }) in
                (&mut currents, &updates).join()
            {
                // this is very lazy and bad, at some point try to keep track of the server clock
                // and then use that to ignore old irrelevant server positions when the 'net cuts
                // out/slows down and let the client physics sim take over then.
                // The challenging part then is just figuring out how to keep clocks synced and
                // figuring out how to factor in ping.
                current.translation.vector = current
                    .translation
                    .vector
                    .lerp(&update.translation.vector, 0.08);

                current.rotation = na::UnitComplex::from_complex(
                    current.rotation.complex()
                        + current.rotation.rotation_to(&update.rotation).complex() * 0.06,
                );
            }
        }
    }

    #[derive(Default)]
    pub struct HandleServerPackets {
        pub server_to_local_ids: HashMap<u32, u32>,
        pub connection_established: bool,
    }
    impl<'a> System<'a> for HandleServerPackets {
        type SystemData = (
            Entities<'a>,
            Read<'a, LazyUpdate>,
            Read<'a, ServerConnection>,
        );

        fn run(&mut self, (ents, lu, sc): Self::SystemData) {
            if let Ok(mut msgs) = sc.message_queue.try_lock() {
                for msg in msgs.drain(0..) {
                    // you know the connection is established when
                    // we first get a message.
                    if !self.connection_established {
                        // immediately request to be put in the game
                        // (later on we might want to have this happen
                        //  after i.e. a menu is clicked through)
                        sc.insert_comp(comn::net::SpawnPlayer);
                        self.connection_established = true;
                    }

                    use NetMessage::*;

                    match msg {
                        NewEnt(server) => {
                            let local: u32 = ents.create().id();
                            self.server_to_local_ids.insert(server, local);
                        }
                        InsertComp(id, net_comp) => {
                            let ent = self
                                .server_to_local_ids
                                .get(&id)
                                .map(|ent| ents.entity(*ent))
                                .filter(|ent| {
                                    if !ents.is_alive(*ent) {
                                        console!(log, "filtering out dead ent");
                                    }
                                    ents.is_alive(*ent)
                                });

                            if let Some(ent) = ent {
                                net_comp.insert(ent, &lu);
                            } else {
                                console!(
                                    error,
                                    "Can't insert component for dead entity",
                                    id,
                                    format!("{:?}", net_comp)
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

mod controls {
    use super::net::ServerConnection;
    use crate::prelude::*;
    use comn::controls::Heading;
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };
    use stdweb::{
        traits::IKeyboardEvent,
        web::{
            document,
            event::{ConcreteEvent, KeyPressEvent, KeyUpEvent},
            IEventTarget,
        },
    };

    //(key direction, key down)
    type KeyMap = Arc<Mutex<HashMap<char, bool>>>;

    pub struct MovementControl {
        keys: KeyMap,
        current_heading: Vec2,
    }
    impl MovementControl {
        fn handle_key_event<K: IKeyboardEvent + ConcreteEvent>(keys: KeyMap, key_down: bool) {
            document().add_event_listener(move |e: K| {
                if !e.repeat() {
                    let first_letter = e.key().chars().next().expect("zero length key name");
                    if "wsad".contains(first_letter) {
                        keys.lock()
                            .expect("Can't lock keys")
                            .insert(first_letter, key_down);
                    }
                }
            });
        }
    }
    impl Default for MovementControl {
        fn default() -> Self {
            let keys = Arc::new(Mutex::new(HashMap::new()));

            Self::handle_key_event::<KeyPressEvent>(keys.clone(), true);
            Self::handle_key_event::<KeyUpEvent>(keys.clone(), false);

            MovementControl {
                keys,
                current_heading: na::zero(),
            }
        }
    }
    impl<'a> System<'a> for MovementControl {
        type SystemData = Read<'a, ServerConnection>;

        fn run(&mut self, sc: Self::SystemData) {
            // if keys isn't being used by the listener,
            if let Ok(keys) = self.keys.try_lock() {
                // these variables are needed to determine direction from key names.
                if keys.len() > 0 {
                    let move_vec = keys.iter().fold(na::zero(), |vec: Vec2, key| match key {
                        ('w', true) => vec - Vec2::y(),
                        ('s', true) => vec + Vec2::y(),
                        ('a', true) => vec - Vec2::x(),
                        ('d', true) => vec + Vec2::x(),
                        _ => vec,
                    });

                    if move_vec != self.current_heading {
                        self.current_heading = move_vec;

                        // now that we know, tell the server where we'd like to go
                        sc.insert_comp(Heading {
                            dir: na::Unit::new_normalize(move_vec),
                        });
                    }
                }
            }
        }
    }
}

fn main() {
    stdweb::initialize();
    // https://github.com/rustwasm/console_error_panic_hook/blob/master/src/lib.rs ?

    // instantiate an ECS world to hold all of the systems, resources, and components.
    let mut world = World::new();

    // add systems and instantiate and order the other systems.
    #[rustfmt::skip]
    let mut dispatcher = DispatcherBuilder::new()
        .with(controls::MovementControl::default(), "move",         &[])
        .with(renderer::Render::default(),          "render",       &[])
        .with(net::HandleServerPackets::default(),  "packets",      &[])
        .with(net::SyncPositions,                   "sync phys",    &[])
        .build();

    // go through all of the systems and register components and resources accordingly
    dispatcher.setup(&mut world);

    fn game_loop(mut dispatcher: specs::Dispatcher<'static, 'static>, mut world: specs::World) {
        // run all of the ECS systems
        dispatcher.dispatch(&mut world);
        world.maintain();

        // tell browser to repeat me the next time the monitor is going to refresh
        window().request_animation_frame(|_| game_loop(dispatcher, world));
    }

    game_loop(dispatcher, world);

    stdweb::event_loop();
}
