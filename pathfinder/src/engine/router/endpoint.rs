//! A struct that represents an endpoint and related data with it.
//!

extern crate config;

use std::collections::{HashMap, HashSet};
use std::sync::{RwLock};

use error::{PathfinderError};
use self::config::{Config, Value};


/// Default AMQP exchange point for requests
pub const REQUEST_EXCHANGE: &'static str = "open-matchmaking.direct";
/// Default AMQP exchange point for responses
pub const RESPONSE_EXCHANGE: &'static str = "open-matchmaking.responses.direct";
/// Type alias for thread-safe endpoint (only for read-only access)
pub type ReadOnlyEndpoint = RwLock<Box<Endpoint>>;

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
    microservice: String,
    request_exchange: String,
    response_exchange: String,
}


impl Endpoint {
    /// Returns a new instance of `Endpoint`.
    pub fn new(url: &str, microservice: &str, request_exchange: &str, response_exchange: &str) -> Endpoint {
        Endpoint {
            url: url.to_string(),
            microservice: microservice.to_string(),
            request_exchange: request_exchange.to_string(),
            response_exchange: response_exchange.to_string()
        }
    }

    /// Returns an original URL for which necessary to do a transformation.
    pub fn get_url(&self) -> String {
        self.url.clone()
    }

    /// Returns a microservice name.
    pub fn get_microservice(&self) -> String {
        self.microservice.clone()
    }

    /// Returns a request exchange point name.
    pub fn get_request_exchange(&self) -> String {
        self.request_exchange.clone()
    }

    /// Returns a response exchange point name.
    pub fn get_response_exchange(&self) -> String {
        self.response_exchange.clone()
    }
}


/// Extract a value configuration object as a string if it exists. Otherwise returns an default value as a string.
fn get_value_from_config_as_str(conf: &HashMap<String, Value>, key: &str, default: &str) -> String {
    match conf.get(key) {
        Some(value) => value.to_owned().into_str().unwrap(),
        None => String::from(default)
    }
}


/// Returns a HashMap with mapping for URL onto certain queue/topic name that
/// were extracted from a configuration.
pub fn extract_endpoints(conf: Box<Config>) -> HashMap<String, ReadOnlyEndpoint> {
    let mut endpoints = HashMap::new();

    let config_endpoints: Vec<Value> = match conf.get_array("endpoints") {
        Ok(array) => array,
        Err(_) => Vec::new()
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
            },
            Err(_) => {
                let error = format!("endpoint \"{}\" is invalid.", endpoint);
                println!("{}", PathfinderError::InvalidEndpoint(error));
                continue;
            }
        };

        // Check on required fields
        let mut missing_fields = Vec::new();
        let required_fields : HashSet<&str> = ["url", "microservice"].iter().cloned().collect();
        for key in required_fields  {
            if !configuration.contains_key(key) {
                missing_fields.push(key);
            }
        }

        if missing_fields.len() > 0 {
            let error = format!("keys {:?} for {} endpoint is missing.", missing_fields, endpoint);
            println!("{}", PathfinderError::InvalidEndpoint(error));
            continue;
        }

        let url = get_value_from_config_as_str(&configuration, "url", "");
        let microservice = get_value_from_config_as_str(&configuration, "microservice", "");
        let request_exchange = get_value_from_config_as_str(&configuration, "request_exchange", &default_request_exchange);
        let response_exchange = get_value_from_config_as_str(&configuration, "response_exchange", &default_response_exchange);
        let endpoint = RwLock::new(Box::new(Endpoint::new(&url, &microservice, &request_exchange, &response_exchange)));
        endpoints.insert(url, endpoint);
    }

    endpoints
}


#[cfg(test)]
mod tests {
    use config::{get_config};
    use engine::router::endpoint::{Endpoint, extract_endpoints};

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
        assert_eq!(endpoints.contains_key("/api/matchmaking/player-of-the-game"), true);
    }

    #[test]
    fn test_extract_endpoints_returns_dict_without_invalid_endpoints() {
        let conf = get_config(&"./tests/files/config_with_invalid_endpoints.yaml");
        let endpoints = extract_endpoints(conf);
        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints.contains_key("/api/matchmaking/player-of-the-game"), true);
    }

    #[test]
    fn test_get_url() {
        let url = "/api/matchmaking/test";
        let microservice_name = "api.matchmaking.test";
        let request_exchange = "open-matchmaking.direct";
        let respone_exchange = "open-matchmaking.responses.direct";
        let endpoint = Endpoint::new(url, microservice_name, request_exchange, respone_exchange);

        assert_eq!(endpoint.get_url(), url);
    }

    #[test]
    fn test_get_microservice_name() {
        let url = "/api/matchmaking/test";
        let microservice_name = "api.matchmaking.test";
        let request_exchange = "open-matchmaking.direct";
        let respone_exchange = "open-matchmaking.responses.direct";
        let endpoint = Endpoint::new(url, microservice_name, request_exchange, respone_exchange);

        assert_eq!(endpoint.get_microservice(), microservice_name);
    }

    #[test]
    fn test_get_request_exchange() {
        let url = "/api/matchmaking/test";
        let microservice_name = "api.matchmaking.test";
        let request_exchange = "open-matchmaking.direct";
        let respone_exchange = "open-matchmaking.responses.direct";
        let endpoint = Endpoint::new(url, microservice_name, request_exchange, respone_exchange);

        assert_eq!(endpoint.get_request_exchange(), request_exchange);
    }

    #[test]
    fn test_get_response_exchange() {
        let url = "/api/matchmaking/test";
        let microservice_name = "api.matchmaking.test";
        let request_exchange = "open-matchmaking.direct";
        let respone_exchange = "open-matchmaking.responses.direct";
        let endpoint = Endpoint::new(url, microservice_name, request_exchange, respone_exchange);

        assert_eq!(endpoint.get_response_exchange(), respone_exchange);
    }
}
