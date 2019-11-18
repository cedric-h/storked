#![recursion_limit = "256"]
#[macro_use]
extern crate stdweb;
use stdweb::web::window;

pub mod prelude {
    pub use super::net::Player;
    pub use comn::prelude::*;
    pub use comn::rmps;
    pub use log::*;
    pub use specs::{prelude::*, Component};
}
use prelude::*;

mod renderer {
    use crate::prelude::*;
    use comn::art::{Animate, AnimationData, Appearance, SpritesheetData, Tile};
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

    pub const ZOOM: f32 = 20.0;
    pub const CANVAS_ZOOM: f32 = 2.0; //change this in renderer.js
    pub const TOTAL_ZOOM: f32 = ZOOM * CANVAS_ZOOM;

    pub struct Render {
        ctx: CanvasContext,
        imgs: HashMap<Appearance, ImageElement>,
    }

    impl Default for Render {
        fn default() -> Self {
            // find the thing we'll draw on
            let canvas: CanvasElement = stdweb::web::document()
                .get_element_by_id("canv")
                .expect("Couldn't find canvas to render on.")
                .try_into()
                .expect("Entity with the 'canv' id isn't a canvas!");

            let ctx = CanvasContext::from_canvas(&canvas)
                .expect("Couldn't get canvas rendering context from canvas");

            ctx.scale(CANVAS_ZOOM as f64, CANVAS_ZOOM as f64);

            // load up the images
            let imgs = Appearance::into_enum_iter()
                .map(|appearance| {
                    let loc = format!("./img/{:?}.png", appearance);

                    // set image up to load
                    let new_img = ImageElement::new();
                    new_img.set_src(&loc);

                    // log on image load
                    js!(@{new_img.clone()}.onload = () => console.log("loaded: ", @{loc}));

                    (appearance, new_img)
                })
                .collect();

            Self { ctx, imgs }
        }
    }

    impl<'a> System<'a> for Render {
        type SystemData = (
            ReadStorage<'a, Appearance>,
            ReadStorage<'a, Pos>,
            ReadStorage<'a, Tile>,
            WriteStorage<'a, Animate>,
        );

        fn run(&mut self, (appearances, poses, tiles, mut animates): Self::SystemData) {
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
                        ((iso.translation.vector.x - SIZE / 2.0) * ZOOM) as f64,
                        ((iso.translation.vector.y - SIZE / 2.0) * ZOOM) as f64,
                        (SIZE * ZOOM) as f64,
                        (SIZE * ZOOM) as f64,
                    )
                    .expect("Couldn't draw tile!");
            }

            // other entities are rendered as if their origin was
            // their center on the X,
            // but their bottom on the Y.
            for (appearance, &Pos(iso), animaybe, _) in
                (&appearances, &poses, (&mut animates).maybe(), !&tiles).join()
            {
                const SIZE: f32 = 2.0;
                if let Some(anim) = animaybe {
                    let SpritesheetData {
                        rows,
                        frame_width,
                        frame_height,
                    } = comn::art::SPRITESHEETS
                        .get(appearance)
                        .unwrap_or_else(|| panic!("No animation data found for {:?}!", appearance));

                    let AnimationData { frame_duration, .. } = rows
                        .get(anim.row)
                        .unwrap_or_else(|| panic!("{:?} has no row #{}!", appearance, anim.row));

                    let current_frame =
                        (anim.current_frame - anim.current_frame % frame_duration) / frame_duration;

                    self.ctx
                        .draw_image_s(
                            self.imgs[appearance].clone(),
                            (frame_width * current_frame) as f64,
                            (frame_height * anim.row) as f64,
                            *frame_width as f64,
                            *frame_height as f64,
                            ((iso.translation.vector.x - SIZE / 2.0) * ZOOM) as f64,
                            ((iso.translation.vector.y - SIZE) * ZOOM) as f64,
                            (SIZE * ZOOM) as f64,
                            (SIZE * ZOOM) as f64,
                        )
                        .expect("Couldn't draw animated non-tile entity!");
                } else {
                    self.ctx
                        .draw_image_d(
                            self.imgs[appearance].clone(),
                            ((iso.translation.vector.x - SIZE / 2.0) * ZOOM) as f64,
                            ((iso.translation.vector.y - SIZE) * ZOOM) as f64,
                            (SIZE * ZOOM) as f64,
                            (SIZE * ZOOM) as f64,
                        )
                        .expect("Couldn't draw non-tile entity!");
                }
            }
        }
    }
}

mod net {
    use crate::prelude::*;
    use bimap::BiMap;
    use comn::{NetComponent, NetMessage, Pos};
    use std::sync::{Arc, Mutex};
    use stdweb::{
        unstable::TryInto,
        web::{
            event::{SocketCloseEvent, SocketErrorEvent, SocketMessageEvent, SocketOpenEvent},
            ArrayBuffer, IEventTarget, WebSocket,
        },
        Value,
    };

    #[derive(Default)]
    pub struct Player(pub Option<Entity>);

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
                info!("Connected to server!");
            });

            ws.add_event_listener(|e: SocketErrorEvent| {
                error!("Errror connecting to {:?}s", e);
            });

            ws.add_event_listener(|e: SocketCloseEvent| {
                error!("Server Connection Closed: {}s", e.reason());
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
            for (
                Pos(Iso2 {
                    translation: at, ..
                }),
                UpdatePosition {
                    iso: Iso2 {
                        translation: go, ..
                    },
                    ..
                },
            ) in (&mut currents, &updates).join()
            {
                /*
                const LERP_DIST: f32 = 0.03;
                let to_go = go.vector - at.vector;

                if to_go.magnitude().abs() > 2.0 * LERP_DIST {
                    at.vector += to_go.normalize() * LERP_DIST;
                } */
                at.vector = at.vector.lerp(&go.vector, 0.03);
                /*
                current.rotation = na::UnitComplex::from_complex(
                    current.rotation.complex()
                        + current.rotation.rotation_to(&update.rotation).complex() * 0.06,
                );*/
            }
        }
    }

    #[derive(Default)]
    pub struct ServerToLocalIds(pub BiMap<u32, u32>);

    #[derive(Default)]
    pub struct HandleServerPackets {
        pub connection_established: bool,
    }
    impl<'a> System<'a> for HandleServerPackets {
        type SystemData = (
            Entities<'a>,
            Write<'a, ServerToLocalIds>,
            Write<'a, Player>,
            Read<'a, LazyUpdate>,
            Read<'a, ServerConnection>,
        );

        fn run(&mut self, (ents, mut server_to_local_ids, mut player, lu, sc): Self::SystemData) {
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
                            server_to_local_ids.0.insert(server, local);
                        }
                        InsertComp(id, net_comp) => {
                            let ent = server_to_local_ids
                                .0
                                .get_by_left(&id)
                                .map(|ent| ents.entity(*ent))
                                .filter(|ent| {
                                    if !ents.is_alive(*ent) {
                                        info!("filtering out dead ent");
                                    }
                                    ents.is_alive(*ent)
                                });

                            if let Some(ent) = ent {
                                match net_comp {
                                    // I should really have some sort of
                                    // Establishment packet that deals with this.
                                    NetComponent::LocalPlayer(_) => {
                                        player.0 = Some(ent);
                                    }
                                    _ => net_comp.insert(ent, &lu),
                                }
                            } else {
                                error!(
                                    "Can't insert component for dead entity, component: {:?}",
                                    net_comp
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
            event::{ConcreteEvent, DoubleClickEvent, KeyPressEvent, KeyUpEvent},
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

            Self {
                keys,
                current_heading: na::zero(),
            }
        }
    }
    impl<'a> System<'a> for MovementControl {
        type SystemData = (
            Read<'a, ServerConnection>,
            Read<'a, Player>,
            WriteStorage<'a, Heading>,
        );

        fn run(&mut self, (sc, player, mut headings): Self::SystemData) {
            // if keys isn't being used by the listener, and the player character has been added.
            if let (Ok(keys), Some(player)) = (self.keys.try_lock(), player.0) {
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
                        let heading = Heading {
                            dir: na::Unit::new_normalize(move_vec),
                        };

                        // now that we know, tell the server where we'd like to go
                        sc.insert_comp(heading.clone());

                        // and record that locally for clientside prediction
                        headings.insert(player, heading.clone()).expect(
                            "couldn't insert heading to player for clientside movement prediction",
                        );
                    }
                }
            }
        }
    }

    pub struct MouseControl {
        mouse_events: Arc<Mutex<Vec<Vec2>>>,
    }
    impl Default for MouseControl {
        fn default() -> Self {
            let mouse_events = Arc::new(Mutex::new(Vec::new()));

            document().add_event_listener({
                use crate::stdweb::traits::IMouseEvent;
                let mouse_events = mouse_events.clone();

                move |e: DoubleClickEvent| {
                    trace!("click!");
                    mouse_events
                        .lock()
                        .expect("Can't lock mouse_events to insert event")
                        .push(Vec2::new(e.client_x() as f32, e.client_y() as f32));
                }
            });

            Self { mouse_events }
        }
    }
    impl<'a> System<'a> for MouseControl {
        type SystemData = (
            Entities<'a>,
            Read<'a, ServerConnection>,
            Read<'a, crate::net::ServerToLocalIds>,
            Read<'a, Player>,
            ReadStorage<'a, Item>,
            ReadStorage<'a, Pos>,
        );

        fn run(&mut self, (ents, sc, server_to_local_ids, player, items, poses): Self::SystemData) {
            use comn::item::{PickupRequest, MAX_INTERACTION_DISTANCE_SQUARED};
            const MAX_ITEM_TO_MOUSE_DISTANCE_SQUARED: f32 = {
                let f = 2.0;
                f * f
            };

            if let (Ok(mut mouse_events), Some(player_entity)) =
                (self.mouse_events.lock(), player.0)
            {
                let Pos(Iso2 {
                    translation: player_translation,
                    ..
                }) = match poses.get(player_entity) {
                    Some(p) => p,
                    // we have a player, but it doesn't have a location yet?
                    // well that's fine, but clicking anything just isn't gonna work.
                    _ => {
                        trace!("can't look for click; player no pos");
                        return;
                    }
                };

                for screen_click in mouse_events.drain(..) {
                    trace!("mouse event!");
                    let click = screen_click / crate::renderer::TOTAL_ZOOM;
                    if let Some(&id) = (&*ents, &poses, &items)
                        .join()
                        // returns (the entity of that item, that item's distance from the mouse)
                        .filter_map(
                            |(
                                item_entity,
                                Pos(Iso2 {
                                    translation: item_translation,
                                    ..
                                }),
                                _,
                            )| {
                                trace!("click detected: {}", click);
                                // first see if the item is close enough to the mouse
                                let item_to_click_distance_squared =
                                    (item_translation.vector - click).magnitude_squared();

                                if item_to_click_distance_squared
                                    < MAX_ITEM_TO_MOUSE_DISTANCE_SQUARED
                                    && ({
                                        trace!("mouse click was close enough to item...");
                                        // if that's true, make sure it's also close enough to the player.
                                        let item_to_player_distance_squared =
                                            (player_translation.vector - item_translation.vector)
                                                .magnitude_squared();

                                        item_to_player_distance_squared
                                            < MAX_INTERACTION_DISTANCE_SQUARED
                                    })
                                {
                                    trace!("click close enough to item and player!");
                                    Some((item_entity, item_to_click_distance_squared))
                                } else {
                                    trace!(
                                        "click too far away: {}\nitem: {}\ndistance: {}",
                                        click,
                                        item_translation.vector,
                                        item_to_click_distance_squared.sqrt()
                                    );
                                    None
                                }
                            },
                        )
                        // finds the item with the shortest distance from the click
                        .min_by(|(_, dist_a), (_, dist_b)| dist_a.partial_cmp(&dist_b).unwrap())
                        // we care about the item's id on the server, not its distance from the player.
                        .and_then(|(item_entity, _)| {
                            server_to_local_ids.0.get_by_right(&item_entity.id())
                        })
                    {
                        trace!("sending request for picking up item with id {}", id);
                        sc.insert_comp(PickupRequest { id });
                    }
                }
            }
        }
    }
}

mod item {
    use crate::prelude::*;
    use comn::art::Appearance;
    use comn::item::{Deposition, DropRequest, Inventory};

    use std::sync::{Arc, Mutex};
    use stdweb::web::{
        document, html_element::ImageElement, IElement, INode, INonElementParentNode,
    };

    pub struct UpdateInventory {
        item_drop_events: Arc<Mutex<Vec<u32>>>,
    }
    impl Default for UpdateInventory {
        fn default() -> Self {
            let item_drop_events = Arc::new(Mutex::new(Vec::new()));

            {
                let item_drop_events = item_drop_events.clone();
                let drop_item = move |id: u64| {
                    item_drop_events
                        .lock()
                        .expect("couldn't lock item drop events")
                        .push(id as u32);
                };
                js! {
                    let drop_item = @{drop_item};
                    $("body").droppable({
                        accept: ".item",
                        drop: function(e, o) {
                            //width: 400px;
                            //height: 225px;
                            if (
                                (o.position.top < -30 || o.position.top > 225 + 30) ||
                                (o.position.left < -30 || o.position.left > 400 + 30)
                            ) {
                                drop_item(Math.floor(o.draggable[0].id));
                            }
                        }
                    });
                }
            }

            Self { item_drop_events }
        }
    }
    impl<'a> System<'a> for UpdateInventory {
        type SystemData = (
            Entities<'a>,
            Read<'a, crate::net::ServerConnection>,
            Read<'a, crate::net::ServerToLocalIds>,
            WriteStorage<'a, Inventory>,
            ReadStorage<'a, Appearance>,
        );

        fn run(
            &mut self,
            (ents, sc, server_to_local_ids, mut inventories, appearances): Self::SystemData,
        ) {
            if let Ok(mut item_drops) = self.item_drop_events.lock() {
                for id in item_drops.drain(..) {
                    sc.insert_comp(DropRequest { id });
                }
            }

            for (ent, inventory) in (&*ents, inventories.drain()).join() {
                let player_id = ent.id().to_string();
                let inventory_div = match document().get_element_by_id(&player_id) {
                    Some(div) => {
                        // clear the items from last time.
                        for _ in 0..div.child_nodes().len() {
                            div.remove_child(&div.first_child().unwrap()).unwrap();
                        }
                        div
                    }
                    None => {
                        let div = document().create_element("div").unwrap();

                        div.class_list().add("box").unwrap();
                        div.class_list().add("inventory").unwrap();
                        div.set_attribute("id", &player_id).unwrap();

                        document().body().unwrap().append_child(&div);

                        js!($("#" + @{player_id}).draggable());
                        div
                    }
                };

                for item_server_id in inventory.items {
                    let item_ent = ents.entity(
                        *server_to_local_ids
                            .0
                            .get_by_left(&item_server_id)
                            .expect("can't render item; invalid server id"),
                    );
                    let appearance = appearances
                        .get(item_ent)
                        .expect("inventory item has no appearance");

                    // set image up to load
                    let new_img = ImageElement::with_size(64, 64);
                    new_img
                        .set_attribute("id", &item_server_id.to_string())
                        .unwrap();
                    new_img.class_list().add("item").unwrap();
                    new_img.set_src(&format!("./img/{:?}.png", appearance));

                    inventory_div.append_child(&new_img);

                    js! {
                        $("#" + @{item_server_id}).draggable({
                            revert: true
                        });
                    }
                }
            }
        }
    }

    /// This system removes the Pos component from entities
    /// with the comn::item::Deposition Component, making the entities
    /// unable to exist physically, effectively turning them into items.
    pub struct DepositionItems;
    impl<'a> System<'a> for DepositionItems {
        type SystemData = (
            Entities<'a>,
            WriteStorage<'a, Deposition>,
            WriteStorage<'a, Pos>,
        );

        fn run(&mut self, (ents, mut deposes, mut poses): Self::SystemData) {
            for (ent, _) in (&*ents, deposes.drain()).join() {
                poses.remove(ent).expect("Couldn't deposition entity");
            }
        }
    }
}

fn main() {
    stdweb::initialize();

    #[cfg(feature = "stdweb-logger")]
    stdweb_logger::init_with_level(Level::Trace);

    // instantiate an ECS world to hold all of the systems, resources, and components.
    let mut world = World::new();

    world.insert(comn::Fps(75.0));

    // add systems and instantiate and order the other systems.
    #[rustfmt::skip]
    let mut dispatcher = DispatcherBuilder::new()
        // controls
        .with(comn::controls::MoveHeadings,         "heading",      &[])
        .with(controls::MovementControl::default(), "move",         &[])
        .with(controls::MouseControl::default(),    "click",        &[])
        // phys
        .with(comn::phys::Collision,                "collision",    &[])
        .with(net::SyncPositions,                   "sync phys",    &[])
        // art
        .with(renderer::Render::default(),          "render",       &[])
        .with(comn::art::UpdateAnimations,          "animate",      &[])
        // util
        .with(net::HandleServerPackets::default(),  "packets",      &[])
        .with(comn::dead::ClearDead,                "clear dead",   &[])
        // items
        .with(item::DepositionItems,                "deposition",   &[])
        .with(item::UpdateInventory::default(),     "update items", &[])
        .build();

    // go through all of the systems and register components and resources accordingly
    dispatcher.setup(&mut world);

    info!("Starting game loop!");

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
