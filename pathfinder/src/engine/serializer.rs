use super::super::error::{Result, PathfinderError};

use json::{parse as parse_json, JsonValue};
use tungstenite::{Message};


pub struct Serializer {
}


impl Serializer {
    pub fn new() -> Serializer {
        Serializer {}
    }

    pub fn serialize(&self, message: String) -> Result<Message> {
        Ok(Message::Text(message))
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

        if json.has_key("microservice") {
            let error_message = String::from("Key `microservice` must be not specified");
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
            "Decoding error: Key `url` is missing or value is `null`"
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
            "Decoding error: Key `url` is missing or value is `null`"
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
            "Decoding error: Key `microservice` must be not specified"
        )
    }
}
