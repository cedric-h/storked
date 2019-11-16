// networking
use std::net::{SocketAddr, TcpListener};
use tungstenite::{accept_hdr, handshake::server::Request, Message};
// util
use crossbeam_channel::{unbounded, Receiver, Sender};
use log::*;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::spawn,
};
// reexports/main lib
use comn::{rmps, specs, Dead, NetComponent, NetMessage};

pub struct ConnectionManager {
    pub from_clients: Receiver<(SocketAddr, NetMessage)>,
    pub to_clients: Sender<(SocketAddr, NetMessage)>,
    pub addr_to_ent: HashMap<SocketAddr, u32>,
}

impl ConnectionManager {
    fn new() -> Self {
        let (to_thread, from_clients) = unbounded();
        let (to_clients, from_thread) = unbounded();

        let msgs_for_srv = to_thread.clone();
        let msgs_to_send = from_thread.clone();
        spawn(move || {
            let server = TcpListener::bind("127.0.0.1:3012").unwrap();

            let channels: Arc<Mutex<HashMap<SocketAddr, Sender<NetMessage>>>> =
                Arc::new(Mutex::new(HashMap::new()));

            spawn({
                let channels = channels.clone();

                move || loop {
                    while let Ok((to_addr, msg)) = msgs_to_send.recv() {
                        // the only other time a lock on this mutex can occur is when
                        // someone is connecting, so theoretically there could be a hitch
                        // then.
                        if let Err(e) =
                            channels.lock().expect("couldn't get channels map")[&to_addr].send(msg)
                        {
                            trace!("couldn't send message to thread for websocket: {}", e);
                        }
                    }
                }
            });

            for stream in server.incoming() {
                debug!("New client connected!");
                let stream = stream.expect("couldn't establish stream.");
                let addr = stream
                    .peer_addr()
                    .expect("unable to determine address of new connector");

                let msgs_for_srv = msgs_for_srv.clone();
                let (channels_s, msgs_to_send) = unbounded();
                channels
                    .lock()
                    .expect("Couldn't get channels map to insert new websocket.")
                    .insert(addr.clone(), channels_s);

                trace!("Sender inserted into channel recorder!");

                spawn(move || {
                    let callback = |req: &Request| {
                        println!("Received a new ws handshake");
                        println!("The request's path is: {}", req.path);
                        println!("The request's headers are:");
                        for &(ref header, _ /* value */) in req.headers.iter() {
                            println!("* {}", header);
                        }

                        // Let's add an additional header to our response to the client.
                        let extra_headers = vec![
                            (String::from("MyCustomHeader"), String::from(":)")),
                            (
                                String::from("SOME_TUNGSTENITE_HEADER"),
                                String::from("header_value"),
                            ),
                        ];
                        Ok(Some(extra_headers))
                    };

                    // this lets us try to get messages from the websocket without blocking.
                    // (so we can send output too)
                    stream.set_nonblocking(true).expect("can't set unblocking");

                    let mut websocket =
                        accept_hdr(stream, callback).expect("couldn't accept handshake");

                    // tell the game thread that a connection with this client has been established.
                    msgs_for_srv
                        .send((
                            addr.clone(),
                            // this 0 is purely filler, clients dont get to pick their ent ofc
                            NetMessage::NewEnt(0),
                        ))
                        .expect(
                            "Couldn't send connection established NewEnt message over channel!",
                        );

                    'poll: loop {
                        if let Ok(Message::Binary(data)) = websocket.read_message() {
                            msgs_for_srv
                                .send((
                                    addr.clone(),
                                    rmps::from_read_ref(&data)
                                        .expect("Couldn't decode NetMessage bytes!"),
                                ))
                                .expect("Couldn't send NetMessage over channel!");
                        }

                        while let Ok(msg) = msgs_to_send.try_recv() {
                            trace!("got {:#?} for {:#?}", msg, addr);

                            // if the call succeeds, all is well, but if it fails we need
                            // to tell the game loop that happened and then stop listening for
                            // their messages because they've probably logged off.
                            if let Err(_) = websocket.write_message(Message::Binary(
                                rmps::encode::to_vec(&msg).expect("Couldn't encode NetMessage!"),
                            )) {
                                // tell the game loop they ded
                                msgs_for_srv
                                    .send((addr.clone(), NetMessage::InsertComp(0, Dead.into())))
                                    .expect("Couldn't send log-off message over channel!");

                                // stop listening for their messages
                                break 'poll;
                            }
                        }
                    }
                });
            }
        });

        Self {
            from_clients,
            to_clients,
            addr_to_ent: HashMap::new(),
        }
    }

    #[inline]
    pub fn send(&self, addr: SocketAddr, msg: NetMessage) {
        self.to_clients
            .send((addr, msg))
            .expect("Couldn't send NetMessage to to_clients channel!");
    }

    #[inline]
    pub fn new_ent(&self, addr: SocketAddr, ent: specs::Entity) {
        self.send(addr, NetMessage::NewEnt(ent.id()));
    }

    #[inline]
    pub fn insert_comp<C: Into<NetComponent>>(
        &self,
        addr: SocketAddr,
        ent: specs::Entity,
        comp: C,
    ) {
        self.send(addr, NetMessage::InsertComp(ent.id(), comp.into()));
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}
