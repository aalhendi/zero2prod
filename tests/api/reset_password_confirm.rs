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

// TODO(aalhendi): invalid password reset token
// TODO(aalhendi): forgot_password_confirm_returns_401_for_expired_token
// TODO(aalhendi): forgot_password_confirm_returns_400_for_missing_token
// TODO(aalhendi): user no longer exists
// TODO(aalhendi): duplicate token attempt
// TODO(aalhendi): invalid password (requirements)
// TODO(aalhendi): password check invalid
// TODO(aalhendi): fatal db error ?
