extern crate config;

use std::collections::{HashMap, HashSet};
use error::PathfinderError;
use self::config::{Config, Value};


fn get_value_from_config_as_str(conf: &HashMap<String, Value>, key: &str) -> String {
    match conf.get(key) {
        Some(value) => value.to_owned().into_str().unwrap(),
        None => String::from("")
    }
}


/// Returns a HashMap so that it contains only mapping from URL into
/// certain Kafka topic.
pub fn extract_endpoints(conf: Box<Config>) -> HashMap<String, String> {
    let mut endpoints = HashMap::new();

    let config_endpoints: Vec<Value> = match conf.get_array("endpoints") {
        Ok(array) => array,
        Err(why) => {
            println!("{}", PathfinderError::SettingsError(why));
            Vec::new()
        }
    };

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
        let mut missing_keys = false;
        let required_fields : HashSet<&str> = ["url", "microservice"].iter().cloned().collect();
        for key in required_fields  {
            if !configuration.contains_key(key) {
                let error = format!("key \"{}\" for {} endpoint is missing.", key, endpoint);
                println!("{}", PathfinderError::InvalidEndpoint(error));
                missing_keys = true;
                break;
            }
        }

        if missing_keys {
            continue;
        }

        let url = get_value_from_config_as_str(&configuration, "url");
        let microservice = get_value_from_config_as_str(&configuration, "microservice");
        endpoints.insert(url, microservice);
    }

    endpoints
}