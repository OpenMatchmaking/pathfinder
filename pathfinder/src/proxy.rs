//use std::cell::{RefCell};
//use std::collections::{HashMap};
//use std::io::{Error, ErrorKind};
//use std::rc::{Rc};
use std::net::SocketAddr;

use super::router::{Router};

use futures::stream::{Stream};
use futures::{Future};
use tokio_core::net::{TcpListener};
use tokio_core::reactor::{Core};
use tungstenite::protocol::{Message};
use tokio_tungstenite::{accept_async};


pub struct Proxy {
    router: Box<Router>
}


impl Proxy {
    pub fn new(router: Box<Router>) -> Proxy {
        Proxy {
            router: router
        }
    }

    pub fn run(&self, address: SocketAddr) {
        let mut core = Core::new().unwrap();
        let handle = core.handle();
        let socket = TcpListener::bind(&address, &handle).unwrap();
        println!("Listening on: {}", address);

        let server = socket.incoming().for_each(|(stream, addr)| {
            Ok(())
        });

        // Run the server
        core.run(server).unwrap();
    }
}
