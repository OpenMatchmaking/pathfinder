use super::error::{Result, PathfinderError};

use json::{parse as parse_json, JsonValue};
use tungstenite::{Message};


pub struct Serializer {
}


impl Serializer {
    pub fn new() -> Serializer {
        Serializer {}
    }

    pub fn deserialize(&self, message: &Message) -> Result<Box<JsonValue>> {
         let text_message = try!(self.parse_into_text(message));
         let mut json_message = try!(self.parse_into_json(text_message.as_str()));
         json_message = try!(self.validate_json(json_message));
         Ok(json_message)
    }

    fn parse_into_text(&self, message: &Message) -> Result<String> {
        match message.clone().into_text() {
            Ok(text_message) => Ok(text_message),
            Err(err) => {
                let formatted_message = format!("{}", err);
                return Err(PathfinderError::DecodingError(formatted_message))
            }
        }
    }

    fn parse_into_json(&self, message: &str) -> Result<Box<JsonValue>> {
        match parse_json(message) {
            Ok(message) => Ok(Box::new(message)),
            Err(err) => {
                let formatted_message = format!("{}", err);
                return Err(PathfinderError::DecodingError(formatted_message))
            }
        }
    }

    fn validate_json(&self, json: Box<JsonValue>) -> Result<Box<JsonValue>> {
        if json["url"].is_null() {
            let error_message = String::from("Key `url` is missing or value is `null`");
            return Err(PathfinderError::DecodingError(error_message));
        }

        if json.has_key("matchmaking") {
            let error_message = String::from("Key `matchmaking` must be not specified");
            return Err(PathfinderError::DecodingError(error_message));
        }

        Ok(json)
    }
}
