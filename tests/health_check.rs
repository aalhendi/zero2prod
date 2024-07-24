use std::net::TcpListener;

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

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let app_address = spawn_app();
    let client = reqwest::Client::new();

    // Act
    let body = "name=john%20doe&email=the_john_doe%40example.com";
    let response = client
        .post(&format!("{app_address}/subscriptions"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(200, response.status().as_u16());
    /*
    To check if side-effects (data storage) have taken place, we can:
    1- Leverage another endpoint of public API to inspect App state
    2- query DB directly in test case
    Option 1 is best to remain fully decoupled and black box BUT we don't want to expose all subscriptions yet...
    */
    // TODO(aalhendi): swap with a public API impl
    
}

#[tokio::test]
// Example of a table-driven / parametrized test. One limitation of "roll-your-own" parametrized tests is
// as soon as one fails, the execution stops and outcome is unknown for remaining test cases.
async fn subscribe_returns_a_400_when_data_is_missing() {
    // Arrange
    let app_address = spawn_app();
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=john%20doe", "missing the email"),
        ("email=the_john_doe%40example.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = client
            .post(&format!("{app_address}/subscriptions"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            // Additional customised error message on test failure
            "The API did not fail with 400 Bad Request when the payload was {error_message}."
        );
    }
}
