use std::collections::{HashMap};

use super::endpoint::{Endpoint};



pub struct Router {
    endpoints: HashMap<String, Box<Endpoint>>
}


impl Router {
    pub fn new(endpoints: HashMap<String, Box<Endpoint>>) -> Router {
        Router {
            endpoints: endpoints
        }
    }
}
