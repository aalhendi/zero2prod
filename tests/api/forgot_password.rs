use crate::helpers::spawn_app;
use sqlx::query;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn forgot_password_returns_200_for_valid_email() {
    // Arrange
    let app = spawn_app().await;
    let body = serde_json::json!({"email": app.test_user.email});

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // Act
    let response = app.post_forgot_password(&body).await;

    // Assert
    assert_eq!(200, response.status().as_u16());
}

// Example: Ensure the reset token is stored for valid user
#[tokio::test]
async fn forgot_password_stores_reset_token_for_valid_user() {
    // Arrange
    let app = spawn_app().await;
    let body = serde_json::json!({"email": app.test_user.email});

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // Act
    let _response = app.post_forgot_password(&body).await;

    // Assert
    let record = query!(
        r#"SELECT reset_token FROM password_resets WHERE user_id = $1"#,
        app.test_user.user_id
    )
    .fetch_one(&app.db_pool)
    .await
    .expect("Failed to fetch password reset token");
    assert!(!record.reset_token.is_empty());
}

// Example: Ensure we do NOT reveal that the user doesn't exist
#[tokio::test]
async fn forgot_password_returns_200_for_nonexistent_email() {
    // Arrange
    let app = spawn_app().await;
    let body = serde_json::json!({"email": "invalid@example.com"});

    // No user inserted with that email

    // Mock email server
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // Act
    let response = app.post_forgot_password(&body).await;

    // Assert
    // We expect a 200 or redirect, but crucially not an error that reveals no user
    assert_eq!(200, response.status().as_u16());
}

// Example: Ensure we actually send an email
#[tokio::test]
async fn forgot_password_sends_email_to_valid_user() {
    // Arrange
    let app = spawn_app().await;
    let body = serde_json::json!({"email": app.test_user.email});

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1) // Expect exactly one request
        .mount(&app.email_server)
        .await;

    // Act
    app.post_forgot_password(&body).await;

    // Assert - wiremock will verify it saw 1 request on drop (because .expect(1))
}

// Example: Test for invalid email in the request body
#[tokio::test]
async fn forgot_password_returns_400_for_invalid_email() {
    // Arrange
    let app = spawn_app().await;
    let invalid_emails = vec!["", "not-an-email-address", "   ", "foo@@bar.com"];

    for email in invalid_emails {
        let body = serde_json::json!({"email": email});

        // Act
        let response = app.post_forgot_password(&body).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request for email: '{email}'"
        );
    }
}

// Example: If DB fails, we get a 500
#[tokio::test]
async fn forgot_password_fails_if_there_is_a_fatal_database_error() {
    // Arrange
    let app = spawn_app().await;
    let body = serde_json::json!({"email": app.test_user.email});

    // We sabotage the table for testing
    sqlx::query!("ALTER TABLE password_resets DROP COLUMN reset_token;")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let response = app.post_forgot_password(&body).await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}
