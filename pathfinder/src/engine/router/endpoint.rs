//! A struct that represents an endpoint and related data with it.
//!

use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;

use config::{Config, Value};
use log::warn;

use crate::engine::{REQUEST_EXCHANGE, RESPONSE_EXCHANGE};
use crate::error::PathfinderError;

/// Type alias for thread-safe endpoint (only for read-only access)
pub type ReadOnlyEndpoint = Arc<Endpoint>;

/// A struct which stores an original URL that must be converted to the
/// certain microservice endpoint.
///
/// # Example
/// ```
/// use engine::router::{Endpoint};
///
/// let endpoint = Endpoint::new(&"/api/matchmaking/search/", &"matchmaking.search");
/// assert_eq!(endpoint.get_url(), String::from("/api/matchmaking/search/"));
/// assert_eq!(endpoint.get_microservice(), String::from("matchmaking.search"));
/// ```
///
#[derive(Debug, Clone)]
pub struct Endpoint {
    url: String,
    routing_key: String,
    request_exchange: String,
    response_exchange: String,
    is_token_required: bool
}

impl Endpoint {
    /// Returns a new instance of `Endpoint`.
    pub fn new(url: &str, routing_key: &str, request_exchange: &str, response_exchange: &str, is_token_required: bool) -> Endpoint {
        Endpoint {
            url: url.to_string(),
            routing_key: routing_key.to_string(),
            request_exchange: request_exchange.to_string(),
            response_exchange: response_exchange.to_string(),
            is_token_required: is_token_required
        }
    }

    /// Returns an original URL for which necessary to do a transformation.
    pub fn get_url(&self) -> String {
        self.url.clone()
    }

    /// Returns a routing key (which can considered as the microservice) name.
    pub fn get_routing_key(&self) -> String {
        self.routing_key.clone()
    }

    /// Returns a request exchange point name.
    pub fn get_request_exchange(&self) -> String {
        self.request_exchange.clone()
    }

    /// Returns a response exchange point name.
    pub fn get_response_exchange(&self) -> String {
        self.response_exchange.clone()
    }

    /// Determines whether to check tokens or not.
    pub fn is_token_required(&self) -> bool {
        self.is_token_required
    }
}

/// Extracts a value configuration object as a string if it exists. Otherwise returns an default 
/// value as a string.
fn get_value_as_str(conf: &HashMap<String, Value>, key: &str, default: &str) -> String {
    match conf.get(key) {
        Some(value) => value.to_owned().into_str().unwrap(),
        None => String::from(default)
    }
}

/// Extracts a value configuration object as a string and tries to convert it to the boolean type. 
/// In the case of parsing errors or when the key doesn't exists returns `false`.
fn get_value_as_bool(conf: &HashMap<String, Value>, key: &str, default: bool) -> bool {
    match conf.get(key) {
        Some(value) => {
            let raw_value = value.to_owned().into_str().unwrap();
            bool::from_str(&raw_value).unwrap_or(default)
        }
        _ => default
    }
}

/// Returns a HashMap with mapping for URL onto certain queue/topic name that
/// were extracted from a configuration.
pub fn extract_endpoints(conf: Box<Config>) -> HashMap<String, ReadOnlyEndpoint> {
    let mut endpoints = HashMap::new();

    let config_endpoints: Vec<Value> = match conf.get_array("endpoints") {
        Ok(array) => array,
        Err(_) => Vec::new(),
    };

    let default_request_exchange = String::from(REQUEST_EXCHANGE);
    let default_response_exchange = String::from(RESPONSE_EXCHANGE);

    for endpoint in &config_endpoints {
        // One the high level you have structure like
        // {"search": {"url": ..., "microservice": ...}},
        // but will be convenient for further processing, if we return
        // {"url": ..., "microservice": ...} instead.
        let configuration = match endpoint.clone().into_table() {
            Ok(table) => {
                let endpoint_name = table.keys().last().unwrap().as_str().clone();
                table[endpoint_name].clone().into_table().unwrap()
            }
            Err(_) => {
                let error = format!("endpoint \"{}\" is invalid.", endpoint);
                warn!("{}", PathfinderError::InvalidEndpoint(error));
                continue;
            }
        };

        // Check on required fields
        let mut missing_fields = Vec::new();
        let required_fields: HashSet<&str> = ["url", "routing_key"].iter().cloned().collect();
        for key in required_fields {
            if !configuration.contains_key(key) {
                missing_fields.push(key);
            }
        }

        if missing_fields.len() > 0 {
            let error = format!(
                "keys {:?} for {} endpoint is missing.",
                missing_fields, endpoint
            );
            warn!("{}", PathfinderError::InvalidEndpoint(error));
            continue;
        }

        let url = get_value_as_str(&configuration, "url", "");
        let routing_key = get_value_as_str(&configuration, "routing_key", "");
        let request_exchange = get_value_as_str(&configuration, "request_exchange", &default_request_exchange);
        let response_exchange = get_value_as_str(&configuration, "response_exchange", &default_response_exchange);
        let is_token_required = get_value_as_bool(&configuration, "token_required", true);
        let endpoint = Endpoint::new(&url, &routing_key, &request_exchange, &response_exchange, is_token_required);
        endpoints.insert(url, Arc::new(endpoint));
    }

    endpoints
}

#[cfg(test)]
mod tests {
    use crate::config::get_config;
    use crate::engine::router::endpoint::{extract_endpoints, Endpoint};

    #[test]
    fn test_extract_endpoints_returns_an_empty_dict_by_default() {
        let conf = get_config(&"");
        let endpoints = extract_endpoints(conf);
        assert_eq!(endpoints.len(), 0);
    }

    #[test]
    fn test_extract_endpoints_returns_an_empty_dict_for_a_file_without_endpoints() {
        let conf = get_config(&"./tests/files/config_without_endpoints.yaml");
        let endpoints = extract_endpoints(conf);
        assert_eq!(endpoints.len(), 0);
    }

    #[test]
    fn test_extract_endpoints_returns_dict_for_a_file_with_valid_endpoints() {
        let conf = get_config(&"./tests/files/config_with_valid_endpoints.yaml");
        let endpoints = extract_endpoints(conf);
        assert_eq!(endpoints.len(), 3);
        assert_eq!(endpoints.contains_key("/api/matchmaking/search"), true);
        assert_eq!(endpoints.contains_key("/api/matchmaking/leaderboard"), true);
        assert_eq!(
            endpoints.contains_key("/api/matchmaking/player-of-the-game"),
            true
        );
    }

    #[test]
    fn test_extract_endpoints_returns_dict_without_invalid_endpoints() {
        let conf = get_config(&"./tests/files/config_with_invalid_endpoints.yaml");
        let endpoints = extract_endpoints(conf);
        assert_eq!(endpoints.len(), 1);
        assert_eq!(
            endpoints.contains_key("/api/matchmaking/player-of-the-game"),
            true
        );
    }

    #[test]
    fn test_get_url() {
        let url = "/api/matchmaking/test";
        let routing_key = "api.matchmaking.test";
        let request_exchange = "open-matchmaking.direct";
        let respone_exchange = "open-matchmaking.responses.direct";
        let endpoint = Endpoint::new(url, routing_key, request_exchange, respone_exchange, false);

        assert_eq!(endpoint.get_url(), url);
    }

    #[test]
    fn test_get_microservice_name() {
        let url = "/api/matchmaking/test";
        let routing_key = "api.matchmaking.test";
        let request_exchange = "open-matchmaking.direct";
        let respone_exchange = "open-matchmaking.responses.direct";
        let endpoint = Endpoint::new(url, routing_key, request_exchange, respone_exchange, false);

        assert_eq!(endpoint.get_routing_key(), routing_key);
    }

    #[test]
    fn test_get_request_exchange() {
        let url = "/api/matchmaking/test";
        let routing_key = "api.matchmaking.test";
        let request_exchange = "open-matchmaking.direct";
        let respone_exchange = "open-matchmaking.responses.direct";
        let endpoint = Endpoint::new(url, routing_key, request_exchange, respone_exchange, false);

        assert_eq!(endpoint.get_request_exchange(), request_exchange);
    }

    #[test]
    fn test_get_response_exchange() {
        let url = "/api/matchmaking/test";
        let routing_key = "api.matchmaking.test";
        let request_exchange = "open-matchmaking.direct";
        let respone_exchange = "open-matchmaking.responses.direct";
        let endpoint = Endpoint::new(url, routing_key, request_exchange, respone_exchange, false);

        assert_eq!(endpoint.get_response_exchange(), respone_exchange);
    }

    #[test]
    fn test_is_token_required_returns_false() {
        let url = "/api/matchmaking/test";
        let routing_key = "api.matchmaking.test";
        let request_exchange = "open-matchmaking.direct";
        let respone_exchange = "open-matchmaking.responses.direct";
        let endpoint = Endpoint::new(url, routing_key, request_exchange, respone_exchange, true);

        assert_eq!(endpoint.is_token_required(), true);
    }

    #[test]
    fn test_is_token_required_returns_true() {
        let url = "/api/matchmaking/test";
        let routing_key = "api.matchmaking.test";
        let request_exchange = "open-matchmaking.direct";
        let respone_exchange = "open-matchmaking.responses.direct";
        let endpoint = Endpoint::new(url, routing_key, request_exchange, respone_exchange, false);

        assert_eq!(endpoint.is_token_required(), false);
    }
}
