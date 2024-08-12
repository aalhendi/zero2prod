use crate::helpers::spawn_app;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let body = "name=john%20doe&email=the_john_doe%40example.com";
    let response = app.post_subscriptions(body).await;

    // Assert
    assert_eq!(200, response.status().as_u16());
    /*
    To check if side-effects (data storage) have taken place, we can:
    1- Leverage another endpoint of public API to inspect App state
    2- query DB directly in test case
    Option 1 is best to remain fully decoupled and black box BUT we don't want to expose all subscriptions yet...
    */
    // TODO(aalhendi): swap with a public API impl
    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscriptions.");

    assert_eq!(saved.email, "the_john_doe@example.com");
    assert_eq!(saved.name, "john doe");
}

#[tokio::test]
// Example of a table-driven / parametrized test. One limitation of "roll-your-own" parametrized tests is
// as soon as one fails, the execution stops and outcome is unknown for remaining test cases.
async fn subscribe_returns_a_400_when_data_is_missing() {
    // Arrange
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=john%20doe", "missing the email"),
        ("email=the_john_doe%40example.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = app.post_subscriptions(invalid_body).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            // Additional customised error message on test failure
            "The API did not fail with 400 Bad Request when the payload was {error_message}."
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    // Arrange
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=&email=the_john_doe%40example.com", "empty name"),
        ("name=John&email=", "empty email"),
        ("name=John&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        // Act
        let response = app.post_subscriptions(body).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 200 OK when the payload was {description}."
        );
    }
}
