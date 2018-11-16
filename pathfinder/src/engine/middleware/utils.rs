/// An extra module that contains handlers for extracting data from
/// responses, that retrieved from a microservices.
///
///  All of these functions extract and provide information as it were
///  defined in according to the next document:
///  https://github.com/OpenMatchmaking/documentation/blob/master/docs/protocol.md
///

use json::JsonValue;


pub fn get_permissions(json: &JsonValue) -> String {
    let mut permissions: Vec<String> = Vec::new();
    for value in json["content"]["permissions"].members() {
        match value.as_str() {
            Some(permission) => permissions.push(String::from(permission)),
            _ => {}
        }
    };
    permissions.join(";")
}
