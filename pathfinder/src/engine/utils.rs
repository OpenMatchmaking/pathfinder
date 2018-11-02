/// Utility module for handling data in Open Matchmaking project.
///
use tungstenite::protocol::Message;

use super::super::error::Result;
use super::serializer::{JsonMessage, Serializer};

/// Transforms an error (which is a string) into JSON object in the special format.
pub fn wrap_an_error(err: &str) -> Message {
    let json_error_message = object!("details" => err);
    let serializer = Serializer::new();
    serializer.serialize(json_error_message.dump()).unwrap()
}

/// Serialize a JSON object into message.
pub fn serialize_message(json: JsonMessage) -> Message {
    let serializer = Serializer::new();
    serializer.serialize(json.dump()).unwrap()
}

/// Deserialize a message into JSON object.
pub fn deserialize_message(message: &Message) -> Result<JsonMessage> {
    let serializer = Serializer::new();
    serializer.deserialize(message)
}

#[cfg(test)]
mod tests {
    use super::super::json::parse as json_parse;
    use super::super::tungstenite::Message;
    use super::{deserialize_message, serialize_message, wrap_an_error};
    use std::sync::Arc;

    #[test]
    fn test_wrap_an_error_returns_json_with_details_field() {
        let error_string = "some error";
        let dictionary = object!{"details" => error_string};
        let expected = Message::Text(dictionary.dump());
        let result = wrap_an_error(error_string);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_serialize_message_returns_a_message_struct() {
        let dictionary = object!{"test" => "value"};
        let test_string = dictionary.dump();
        let raw_data = Arc::new(Box::new(json_parse(&test_string).unwrap()));
        let result = serialize_message(raw_data);

        assert_eq!(result.is_text(), true)
    }

    #[test]
    fn test_deserialize_message_returns_a_json_message() {
        let dictionary = object!{"url" => "test"};
        let message = Message::Text(dictionary.dump());
        let result = deserialize_message(&message);

        assert_eq!(result.is_ok(), true);
        let unwrapped_result = result.unwrap();
        assert_eq!(unwrapped_result.has_key("url"), true);
        assert_eq!(unwrapped_result["url"], dictionary["url"]);
    }

    #[test]
    fn test_deserialize_message_returns_an_error() {
        let invalid_json = String::from(r#"{"url": "test""#);
        let message = Message::Text(invalid_json);
        let result = deserialize_message(&message);

        assert_eq!(result.is_err(), true);
        assert_eq!(
            format!("{}", result.unwrap_err()),
            "Decoding error: Unexpected end of JSON"
        )
    }
}
