use std::net::TcpListener;

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let address = spawn_app();
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(format!("{address}/health_check"))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert!(response.content_length().is_some_and(|length| length == 0));
}

// This is done to fully decouple test suite from underlying implementation details.
// We test the exact same way a user would (black box testing) to avoid user-visible regression
fn spawn_app() -> String {
    // No need to propagate errors, we're running tests.
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port");
    // Port 0 is special-cased at OS level. OS will scan for available port and bind app to it
    let port = listener.local_addr().unwrap().port();
    let server = zero2prod::run(listener).expect("Failed to bind address");
    // Launch server as background task. We don't need the handle, so its discarded.
    #[allow(clippy::let_underscore_future)]
    let _ = tokio::spawn(server);
    // tokio::test spins up a new runtime at beginning of each test case & shuts it down at the end of each test case.
    // when a tokio runtime is shut down all tasks spawned on it are dropped.

    format!("http://127.0.0.1:{port}")
}
