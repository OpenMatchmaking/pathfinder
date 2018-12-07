//! Routing entity for handling endpoints
//!
//! This module is intended for matching and converting a passed URLs by a client
//! in request into certain queue/topic names.
//!

use std::clone::Clone;
use std::collections::HashMap;

use crate::engine::router::endpoint::ReadOnlyEndpoint;
use crate::error::{PathfinderError, Result};

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
    endpoints: HashMap<String, ReadOnlyEndpoint>
}

impl Router {
    /// Returns a new instance of `Router` that contains a mapping for resources.
    pub fn new(endpoints: HashMap<String, ReadOnlyEndpoint>) -> Router {
        Router {
            endpoints: endpoints
        }
    }

    /// Returns an endpoint that was found for a passed URL.
    pub fn match_url(&self, url: &str) -> Result<ReadOnlyEndpoint> {
        match self.endpoints.contains_key(url) {
            true => {
                let endpoint = self.endpoints[url].clone();
                Ok(endpoint)
            }
            false => Err(PathfinderError::EndpointNotFound(url.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::get_config;
    use crate::engine::router::{extract_endpoints, Router};

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
        assert_eq!(endpoint.get_routing_key(), "microservice.search");
    }

    #[test]
    fn test_routers_match_url_returns_an_error_for_an_unknown_url() {
        let router = get_router(&"./tests/files/config_with_invalid_endpoints.yaml");
        let result_match = router.match_url(&"/api/matchmaking/search");

        assert_eq!(result_match.is_err(), true);
    }
}
