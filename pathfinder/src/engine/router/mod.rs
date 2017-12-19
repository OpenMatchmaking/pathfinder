pub mod endpoint;

pub use self::endpoint::{Endpoint, extract_endpoints};

use std::collections::{HashMap};
use std::clone::{Clone};

use super::super::error::{Result, PathfinderError};


pub struct Router {
    endpoints: HashMap<String, Box<Endpoint>>
}


impl Router {
    pub fn new(endpoints: HashMap<String, Box<Endpoint>>) -> Router {
        Router {
            endpoints: endpoints
        }
    }

    pub fn match_url(&self, url: &str) -> Result<Box<Endpoint>> {
        match self.endpoints.contains_key(url) {
            true => Ok(self.endpoints[url].clone()),
            false => Err(PathfinderError::EndpointNotFound(url.to_string()))
        }
    }
}


#[cfg(test)]
mod tests {
    use config::{get_config};
    use engine::router::{Router, Endpoint, extract_endpoints};
    use error::{Result};

    fn get_route(file_path: &str, url: &str) -> Result<Box<Endpoint>> {
        let config = get_config(file_path);
        let endpoints = extract_endpoints(config);
        let router = Box::new(Router::new(endpoints));
        router.match_url(url)
    }

    #[test]
    fn test_router_returns_endpoint_for_a_match() {
        let route = get_route(
            &"./tests/files/config_with_valid_endpoints.yaml",
            "/api/matchmaking/search"
        );

        assert_eq!(route.is_ok(), true);
        let endpoint = route.unwrap();
        assert_eq!(endpoint.get_url(), "/api/matchmaking/search");
        assert_eq!(endpoint.get_microservice(), "microservice.search");
    }

    #[test]
    fn test_router_returns_an_error_for_non_existing_endpoint() {
        let route = get_route(
            &"./tests/files/config_with_invalid_endpoints.yaml",
            "/api/matchmaking/search"
        );

        assert_eq!(route.is_err(), true);
    }
}
