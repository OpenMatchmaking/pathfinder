/// Utility module for handling data in Open Matchmaking project.
///

use tungstenite::protocol::{Message};

use super::error::{Result};
use super::serializer::{Serializer, JsonMessage};


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
