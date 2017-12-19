// Utility wrappers around an engine instance, so that it simplify an
// existing codebase and makes it more readable


#[macro_export]
macro_rules! handle_error {
    (&$connections:expr, &$addr:expr, $engine_instance:expr, $error:expr) => {
        let transmitter = &$connections.borrow_mut()[&$addr];
        let formatted_error = format!("{}", $error);
        let error_message = $engine_instance.borrow().wrap_an_error(formatted_error.as_str());
        transmitter.unbounded_send(error_message).unwrap();
    };
}
