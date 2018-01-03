//! Macros for a work with `engine::Engine`
//!
//! Utility wrappers around an instance of the `engine::Engine` type, so that
//! it simplify an existing codebase and makes it more readable.
//!

/// Sends an error message in JSON format to a client via `UnboundedSender`,
/// being a part of `mpsc::UnboundedSender<Message>` object.
///
/// # Remarks
///
/// This macro extracting a transmitter of the `UnboundedSender` type from the
/// passed `connections` argument. After the passed `error` transforms into
/// `std::string::String` so that it can be sent in JSON format later.
///
/// # Arguments
/// * `connections` - A reference to a variable of the `std::collections::HashMap<SocketAddr, mpsc::UnboundedSender<Message>` type.
/// * `addr` - A reference to an instance of the `std::net::SocketAddr` type.
/// * `engine_instance` - An instance of the `engine::{Engine}` type.
/// * `error` - An instance of `error::Error`, that will be processed and sent to a client.
///
#[macro_export]
macro_rules! handle_error {
    (&$connections:expr, &$addr:expr, $engine_instance:expr, $error:expr) => {
        let transmitter = &$connections.borrow_mut()[&$addr];
        let formatted_error = format!("{}", $error);
        let error_message = $engine_instance.borrow().wrap_an_error(formatted_error.as_str());
        transmitter.unbounded_send(error_message).unwrap();
    };
}
