//! Routing entity for handling endpoints
//!
//! This module is intended for matching and converting a passed URLs by a client
//! in request into certain queue/topic names.
//!

pub mod endpoint;

pub use self::endpoint::{Endpoint, extract_endpoints, REQUEST_EXCHANGE, RESPONSE_EXCHANGE};

use std::collections::{HashMap};
use std::clone::{Clone};
use std::rc::{Rc};

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
/// For cases when necessary to return an endpoint that bases on the value,
/// that wasn't specified in configuration file, you can do this via
/// using `match_url_or_default` method:
///
/// ```
/// use config::{get_config};
/// use engine::router::{Router, Endpoint, extract_endpoints};
///
/// let url = "/api/matchmaking/rewards";
/// let config = get_config(&"../../../tests/files/config_with_valid_endpoints.yaml");
/// let endpoints = extract_endpoints(config);
/// let router = Box::new(Router::new(endpoints))
///
/// let endpoint = route.match_url_or_default(url);
/// assert_eq!(endpoint.get_url(), "/api/matchmaking/rewards");
/// assert_eq!(endpoint.get_microservice(), "api.matchmaking.rewards");
/// ```
///
pub struct Router {
    endpoints: HashMap<String, Rc<Box<Endpoint>>>
}


impl Router {
    /// Returns a new instance of `Router` that contains a mapping for resources.
    pub fn new(endpoints: HashMap<String, Rc<Box<Endpoint>>>) -> Router {
        Router {
            endpoints: endpoints
        }
    }

    /// Returns an endpoint that was found for a passed URL.
    pub fn match_url(&self, url: &str) -> Result<Rc<Box<Endpoint>>> {
        match self.endpoints.contains_key(url) {
            true => Ok(self.endpoints[url].clone()),
            false => Err(PathfinderError::EndpointNotFound(url.to_string()))
        }
    }

    /// Returns an endpoint for the matched URL. If wasn't found returns a processed
    /// URL as endpoint like in normal cases.
    pub fn match_url_or_default(&self, url: &str) -> Rc<Box<Endpoint>> {
        match self.match_url(url) {
            Ok(endpoint) => endpoint,
            Err(_) => {
                let url = url.to_string();
                let microservice = self.convert_url_into_microservice(&url);
                Rc::new(Box::new(Endpoint::new(&url, &microservice, REQUEST_EXCHANGE, RESPONSE_EXCHANGE)))
            }
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
    use engine::router::{Router, extract_endpoints};

    fn get_router(file_path: &str) -> Box<Router> {
        let config = get_config(file_path);
        let endpoints = extract_endpoints(config);
        Box::new(Router::new(endpoints))
    }

    #[test]
    fn test_router_match_url_returns_an_endpoint_for_a_matched_url() {
        let router = get_router(&"./tests/files/config_with_valid_endpoints.yaml");
        let result_match = router.match_url(&"/api/matchmaking/search");

        assert_eq!(result_match.is_ok(), true);
        let endpoint = result_match.unwrap();
        assert_eq!(endpoint.get_url(), "/api/matchmaking/search");
        assert_eq!(endpoint.get_microservice(), "microservice.search");
    }

    #[test]
    fn test_routers_match_url_returns_an_error_for_an_unknown_url() {
        let router = get_router(&"./tests/files/config_with_invalid_endpoints.yaml");
        let result_match = router.match_url(&"/api/matchmaking/search");

        assert_eq!(result_match.is_err(), true);
    }

    #[test]
    fn test_router_match_url_or_default_returns_an_existing_endpoint_for_a_matched_url()
    {
        let router = get_router(&"./tests/files/config_with_valid_endpoints.yaml");
        let endpoint = router.match_url_or_default(&"/api/matchmaking/search");

        assert_eq!(endpoint.get_url(), "/api/matchmaking/search");
        assert_eq!(endpoint.get_microservice(), "microservice.search");
    }

    #[test]
    fn test_router_returns_a_default_match_return_a_custom_endpoint_for_an_unknown_url()
    {
        let router = get_router(&"./tests/files/config_with_valid_endpoints.yaml");
        let endpoint = router.match_url_or_default(&"/api/matchmaking/unknown");

        assert_eq!(endpoint.get_url(), "/api/matchmaking/unknown");
        assert_eq!(endpoint.get_microservice(), "api.matchmaking.unknown");
    }
}
