use crate::helpers::{assert_is_redirect_to, extract_token, spawn_app};
use uuid::Uuid;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};
use zero2prod::domain::PasswordResetToken;

#[tokio::test]
async fn reset_password_confirm_succeeds_for_valid_token() {
    // Arrange
    let app = spawn_app().await;
    let password = Uuid::new_v4().to_string();

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let reset_password_body = serde_json::json!({"email": app.test_user.email});
    let reset_password_response = app.post_reset_password(&reset_password_body).await;
    assert_is_redirect_to(&reset_password_response, "/login");

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let reset_links = app.get_confirmation_links(email_request);
    let token = PasswordResetToken::parse(extract_token(&reset_links.plain_text, "token")).unwrap();

    let reset_get_response = app.get_password_reset_confirm(&token).await;
    assert_eq!(200, reset_get_response.status().as_u16());

    // Act
    let confirm_body = serde_json::json!({"token": token.as_ref(),"new_password": &password,"new_password_check": &password});
    let response = app.post_reset_password_confirm(&confirm_body).await;

    // Assert
    assert_is_redirect_to(&response, "/login");

    // Act - Part 2 - Follow the redirect
    let html_page = app.get_login_html().await;

    // Assert
    assert!(html_page.contains("<p><i>Your password has been reset.</i></p>"));

    // Act - Part 3 - Login using the new password
    let login_payload = serde_json::json!({
    "username": &app.test_user.username,
    "password": &password
    });
    let response = app.post_login(&login_payload).await;

    // Assert
    assert_is_redirect_to(&response, "/admin/dashboard");
}

#[tokio::test]
async fn clicking_on_password_reset_confirmation_link_returns_200() {
    // Arrange
    let app = spawn_app().await;
    let body = serde_json::json!({"email": app.test_user.email});

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_reset_password(&body).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(email_request);

    // Act
    let response = reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    // Assert
    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn reset_password_confirm_returns_400_for_invalid_token() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let invalid_token = Uuid::new_v4().to_string();

    // Act
    let confirm_body = serde_json::json!({
        "token": invalid_token,
        "new_password": &new_password,
        "new_password_check": &new_password
    });
    let response = app.post_reset_password_confirm(&confirm_body).await;

    // Assert
    assert_eq!(400, response.status().as_u16());
    assert!(response
        .text()
        .await
        .unwrap()
        .contains("Invalid or expired password reset token."));
}

#[tokio::test]
async fn reset_password_confirm_returns_400_for_expired_token() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_reset_password(&serde_json::json!({"email": app.test_user.email}))
        .await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let reset_links = app.get_confirmation_links(email_request);
    let token = PasswordResetToken::parse(extract_token(&reset_links.plain_text, "token")).unwrap();

    // Expire the token
    sqlx::query!(
        "UPDATE password_resets SET expires_at = now() - interval '1 hour' WHERE user_id = $1",
        app.test_user.user_id
    )
    .execute(&app.db_pool)
    .await
    .unwrap();

    // Act
    let confirm_body = serde_json::json!({
        "token": token.as_ref(),
        "new_password": &new_password,
        "new_password_check": &new_password
    });
    let response = app.post_reset_password_confirm(&confirm_body).await;

    // Assert
    assert_eq!(400, response.status().as_u16());
    assert!(response
        .text()
        .await
        .unwrap()
        .contains("Invalid or expired password reset token."));
}

#[tokio::test]
async fn reset_password_confirm_returns_400_for_missing_token() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    // Act
    let confirm_body = serde_json::json!({
        "new_password": &new_password,
        "new_password_check": &new_password
    });
    let response = app.post_reset_password_confirm(&confirm_body).await;

    // Assert
    assert_eq!(400, response.status().as_u16());
}

#[tokio::test]
async fn reset_password_confirm_returns_400_after_user_deletion() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_reset_password(&serde_json::json!({"email": app.test_user.email}))
        .await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let reset_links = app.get_confirmation_links(email_request);
    let token = PasswordResetToken::parse(extract_token(&reset_links.plain_text, "token")).unwrap();

    // Delete the user (token is cascade-deleted)
    sqlx::query!(
        "DELETE FROM users WHERE user_id = $1",
        app.test_user.user_id
    )
    .execute(&app.db_pool)
    .await
    .unwrap();

    // Act
    let confirm_body = serde_json::json!({
        "token": token.as_ref(),
        "new_password": &new_password,
        "new_password_check": &new_password
    });
    let response = app.post_reset_password_confirm(&confirm_body).await;

    // Assert
    assert_eq!(400, response.status().as_u16());
}

#[tokio::test]
async fn reset_password_confirm_returns_400_for_duplicate_token_attempt() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_reset_password(&serde_json::json!({"email": app.test_user.email}))
        .await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let reset_links = app.get_confirmation_links(email_request);
    let token = PasswordResetToken::parse(extract_token(&reset_links.plain_text, "token")).unwrap();

    // First attempt (success)
    let confirm_body = serde_json::json!({
        "token": token.as_ref(),
        "new_password": &new_password,
        "new_password_check": &new_password
    });
    let response = app.post_reset_password_confirm(&confirm_body).await;
    assert_is_redirect_to(&response, "/login");

    // Second attempt
    let response = app.post_reset_password_confirm(&confirm_body).await;

    // Assert
    assert_eq!(400, response.status().as_u16());
    assert!(response
        .text()
        .await
        .unwrap()
        .contains("Invalid or expired password reset token."));
}

#[tokio::test]
async fn reset_password_confirm_returns_error_for_invalid_password() {
    // Arrange
    let app = spawn_app().await;
    let new_password = "invalid"; // Invalid password

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_reset_password(&serde_json::json!({"email": app.test_user.email}))
        .await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let reset_links = app.get_confirmation_links(email_request);
    let token = PasswordResetToken::parse(extract_token(&reset_links.plain_text, "token")).unwrap();

    // Act
    let confirm_body = serde_json::json!({
        "token": token.as_ref(),
        "new_password": new_password,
        "new_password_check": new_password
    });
    let response = app.post_reset_password_confirm(&confirm_body).await;

    // Assert
    assert_is_redirect_to(
        &response,
        &format!(
            "/password-reset/confirm?token={token}",
            token = token.as_ref()
        ),
    );

    let response = app.get_password_reset_confirm(&token).await;
    let html_page = response.text().await.unwrap();
    assert!(html_page.contains("Password must be 8 characters or longer."));
}

#[tokio::test]
async fn reset_password_confirm_returns_error_for_password_mismatch() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let different_password = Uuid::new_v4().to_string();

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_reset_password(&serde_json::json!({"email": app.test_user.email}))
        .await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let reset_links = app.get_confirmation_links(email_request);
    let token = PasswordResetToken::parse(extract_token(&reset_links.plain_text, "token")).unwrap();

    // Act
    let confirm_body = serde_json::json!({
        "token": token.as_ref(),
        "new_password": &new_password,
        "new_password_check": &different_password
    });
    let response = app.post_reset_password_confirm(&confirm_body).await;

    // Assert
    assert_is_redirect_to(
        &response,
        &format!(
            "/password-reset/confirm?token={token}",
            token = token.as_ref()
        ),
    );

    // Check flash message
    let response = app.get_password_reset_confirm(&token).await;
    let html_page = response.text().await.unwrap();
    assert!(html_page
        .contains("You entered two different new passwords - the field values must match."));
}

#[tokio::test]
async fn reset_password_confirm_returns_500_for_fatal_database_error() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_reset_password(&serde_json::json!({"email": app.test_user.email}))
        .await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let reset_links = app.get_confirmation_links(email_request);
    let token = PasswordResetToken::parse(extract_token(&reset_links.plain_text, "token")).unwrap();

    // Sabotage the database by dropping the `password_resets` table
    sqlx::query!("DROP TABLE password_resets")
        .execute(&app.db_pool)
        .await
        .unwrap();

    // Act
    let confirm_body = serde_json::json!({
        "token": token.as_ref(),
        "new_password": &new_password,
        "new_password_check": &new_password
    });
    let response = app.post_reset_password_confirm(&confirm_body).await;

    // Assert
    assert_eq!(500, response.status().as_u16());
}
