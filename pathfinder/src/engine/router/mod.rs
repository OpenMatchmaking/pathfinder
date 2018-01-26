//! Routing entity for handling endpoints
//!
//! This module is intended for matching and converting a passed URLs by a client
//! in request into certain queue/topic names.
//!

pub mod endpoint;

pub use self::endpoint::{Endpoint, extract_endpoints};

use std::collections::{HashMap};
use std::clone::{Clone};

use super::super::error::{Result, PathfinderError};


/// A struct which is stores a mapping of resources that can be
/// represented in requests and transformed into certain API endpoints of
/// microservices.
///
/// # Examples
///
/// Attempt to get an endpoint with correct URL will return all expected data:
///
/// ```
/// use config::{get_config};
/// use engine::router::{Router, Endpoint, extract_endpoints};
///
/// let url = "/api/matchmaking/search";
/// let config = get_config(&"../../../tests/files/config_with_valid_endpoints.yaml");
/// let endpoints = extract_endpoints(config);
/// let router = Box::new(Router::new(endpoints))
///
/// let endpoint = route.match_url(url).unwrap();
/// assert_eq!(endpoint.get_url(), "/api/matchmaking/search");
/// assert_eq!(endpoint.get_microservice(), "microservice.search");
/// ```
///
/// For not matched URL will be returned an error:
///
/// ```
/// use config::{get_config};
/// use engine::router::{Router, Endpoint, extract_endpoints};
///
/// let url = "/api/matchmaking/search";
/// let config = get_config(&"../../../tests/files/config_with_invalid_endpoints.yaml");
/// let endpoints = extract_endpoints(config);
/// let router = Box::new(Router::new(endpoints))
///
/// assert_eq!(route.match_url(url).is_err(), true);
/// ```
///
pub struct Router {
    endpoints: HashMap<String, Box<Endpoint>>
}


impl Router {
    /// Returns a new instance of `Router` that contains a mapping for resources.
    pub fn new(endpoints: HashMap<String, Box<Endpoint>>) -> Router {
        Router {
            endpoints: endpoints
        }
    }

    /// Returns an endpoint that was found for a passed URL.
    pub fn match_url(&self, url: &str) -> Result<Box<Endpoint>> {
        match self.endpoints.contains_key(url) {
            true => Ok(self.endpoints[url].clone()),
            false => Err(PathfinderError::EndpointNotFound(url.to_string()))
        }
    }

    /// Returns an endpoint for the matched URL. If wasn't found returns a processed
    /// URL as endpoint like in normal cases.
    pub fn match_url_or_default(&self, url: &str) -> Box<Endpoint> {
        match self.match_url(url) {
            Ok(endpoint) => endpoint,
            Err(_) => Box::new(Endpoint {
                url: url.to_string(),
                microservice: self.convert_url_into_microservice(url)
            })
        }
    }

    /// Converts a URL to the certain microservice name, so that it will be used as a
    /// queue/topic name further.
    fn convert_url_into_microservice(&self, url: &str) -> String {
        let mut external_url = url.clone();
        external_url = external_url.trim_left_matches("/");
        external_url = external_url.trim_right_matches("/");
        external_url.replace("/", ".")
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
