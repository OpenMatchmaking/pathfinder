//! Serializer and deserializer structure for a data.
//!
//! This module is intended for transforming incoming data (which are
//! `tungstenite::Message` objects) to JSON objects and preparing responses
//! for client before sending through transmitters.
//!

use std::sync::{Arc};

use error::{Result, PathfinderError};

use json::{parse as parse_json, JsonValue};
use tungstenite::protocol::{Message};


/// Type alias for JSON object
pub type JsonMessage = Arc<Box<JsonValue>>;

/// A specialized struct for deserializing incoming messages into JSON and
/// serializing responses into `tungstenite::Message` objects, so, that they
/// could be send to a client.
///
/// # Examples
///
/// Serializing a JSON object into message:
///
/// ```
/// use engine::{Serializer};
///
/// let instance = Serializer::new();
/// let json = object!{"test" => "serialize"};
/// let response = json.dump();
/// println!("{:?}", instance.serialize(response))
/// ```
///
/// Deserializing a message to JSON object:
///
/// ```
/// use engine::{Serializer};
/// use tungstenite::{Message};
///
/// let json = object!{"test" => "serialize"};
/// let message = Message::Text(json.dump());
/// let instance = Serializer::new();
/// println!("{:?}", instance.deserialize(&message))
/// ```
///
pub struct Serializer;


impl Serializer {
    /// Returns a new instance of `Serializer`.
    pub fn new() -> Serializer {
        Serializer {}
    }

    /// Converts a UTF-8 encoded `std::string::String` into an instance of
    /// the `tungstenite::Message` type, so that this message can be send to
    /// a client.
    pub fn serialize(&self, message: String) -> Result<Message> {
        Ok(Message::Text(message))
    }

    /// Transforms an instance of the `tungstenite::Message` type into JSON object.
    pub fn deserialize(&self, message: &Message) -> Result<JsonMessage> {
         let text_message = self.parse_into_text(message)?;
         let mut json_message = self.parse_into_json(text_message.as_str())?;
         json_message = self.validate_json(json_message)?;
         Ok(json_message)
    }

    /// Parses an instance of the `tungstenite::Message` type and returns a UTF-8
    /// encoded string of the `std::string::String` type.
    fn parse_into_text(&self, message: &Message) -> Result<String> {
        match message.clone().into_text() {
            Ok(text_message) => Ok(text_message),
            Err(err) => {
                let formatted_message = format!("{}", err);
                return Err(PathfinderError::DecodingError(formatted_message))
            }
        }
    }

    /// Parses a UTF-8 string and converts it into JSON object.
    fn parse_into_json(&self, message: &str) -> Result<JsonMessage> {
        match parse_json(message) {
            Ok(message) => Ok(Arc::new(Box::new(message))),
            Err(err) => {
                let formatted_message = format!("{}", err);
                return Err(PathfinderError::DecodingError(formatted_message))
            }
        }
    }

    /// Validates a JSON object on required fields and values.
    fn validate_json(&self, json: JsonMessage) -> Result<JsonMessage> {
        if json["url"].is_null() {
            let error_message = String::from("The `url` field is missing or value is `null`");
            return Err(PathfinderError::DecodingError(error_message));
        }

        if json.has_key("microservice") {
            let error_message = String::from("The `microservice` field must not be specified");
            return Err(PathfinderError::DecodingError(error_message));
        }

        Ok(json)
    }
}



#[cfg(test)]
mod tests {
    use super::{Serializer};
    use super::super::json::{Null};
    use super::super::tungstenite::{Message};

    #[test]
    fn test_serialize_returns_a_new_message_instance() {
        let instance = Serializer::new();
        let dictionary = object!{"test" => "value"};
        let test_string = dictionary.dump();
        let result = instance.serialize(test_string);

        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap().is_text(), true)
    }

    #[test]
    fn test_deserialize_returns_valid_json_object() {
        let dictionary = object!{"url" => "test"};
        let message = Message::Text(dictionary.dump());
        let instance = Serializer::new();
        let result = instance.deserialize(&message);

        assert_eq!(result.is_ok(), true);
        let unwrapped_result = result.unwrap();
        assert_eq!(unwrapped_result.has_key("url"), true);
        assert_eq!(unwrapped_result["url"], dictionary["url"]);
    }

    #[test]
    fn test_deserialize_returns_decoding_error_while_parsed_into_text() {
        let data = vec![0, 159, 146, 150];
        let message = Message::Binary(data);
        let instance = Serializer::new();
        let result = instance.deserialize(&message);

        assert_eq!(result.is_err(), true);
        assert_eq!(
            format!("{}", result.unwrap_err()),
            "Decoding error: UTF-8 encoding error"
        )
    }

    #[test]
    fn test_deserialize_returns_decoding_error_while_parsed_into_json() {
        let invalid_json = String::from(r#"{"url": "test""#);
        let message = Message::Text(invalid_json);
        let instance = Serializer::new();
        let result = instance.deserialize(&message);

        assert_eq!(result.is_err(), true);
        assert_eq!(
            format!("{}", result.unwrap_err()),
            "Decoding error: Unexpected end of JSON"
        )
    }

    #[test]
    fn test_deserialize_returns_validation_error_for_missing_url_key_in_json() {
        let dictionary = object!{"test" => "value"};
        let message = Message::Text(dictionary.dump());
        let instance = Serializer::new();
        let result = instance.deserialize(&message);

        assert_eq!(result.is_err(), true);
        assert_eq!(
            format!("{}", result.unwrap_err()),
            "Decoding error: The `url` field is missing or value is `null`"
        )
    }

    #[test]
    fn test_deserialize_returns_validation_error_for_invalid_url_value_in_json() {
        let dictionary = object!{"url" => Null};
        let message = Message::Text(dictionary.dump());
        let instance = Serializer::new();
        let result = instance.deserialize(&message);

        assert_eq!(result.is_err(), true);
        assert_eq!(
            format!("{}", result.unwrap_err()),
            "Decoding error: The `url` field is missing or value is `null`"
        )
    }

    #[test]
    fn test_deserialize_returns_validation_error_for_the_specified_matchmaking_key_in_json() {
        let dictionary = object!{"url" => "value", "microservice" => "some microservice"};
        let message = Message::Text(dictionary.dump());
        let instance = Serializer::new();
        let result = instance.deserialize(&message);

        assert_eq!(result.is_err(), true);
        assert_eq!(
            format!("{}", result.unwrap_err()),
            "Decoding error: The `microservice` field must not be specified"
        )
    }
}
