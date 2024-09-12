use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

use crate::helpers::{assert_is_redirect_to, spawn_app, ConfirmationLinks, TestApp};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;
    app.test_user.login(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0) // no request is fired at Postmark!
        .mount(&app.email_server)
        .await;

    // Act - Part 1 - Submit newsletter form
    let newsletter_request_body = serde_json::json!({
    "title": "Newsletter title",
    "text_content": "Newsletter body as plain text",
    "html_content": "<p>Newsletter body as HTML</p>",
    "idempotency_key": uuid::Uuid::new_v4().to_string()
    });
    let response = app.post_publish_newsletter(newsletter_request_body).await;

    // Assert
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act - Part 2 - Follow the redirect
    let html_page = app.get_publish_newsletter_html().await;

    // Assert
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));

    // Mock verifies on Drop that we haven't sent the newsletter email
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.test_user.login(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act - Part 1 - Submit newsletter form
    let newsletter_request_body = serde_json::json!({
    "title": "Newsletter title",
    "text_content": "Newsletter body as plain text",
    "html_content": "<p>Newsletter body as HTML</p>",
    "idempotency_key": uuid::Uuid::new_v4().to_string()
    });
    let response = app.post_publish_newsletter(newsletter_request_body).await;

    // Assert
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act - Part 2 - Follow the redirect
    let html_page = app.get_publish_newsletter_html().await;

    // Assert
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));

    // Mock verifies on Drop that we have sent the newsletter email
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    // Arrange
    let app = spawn_app().await;
    app.test_user.login(&app).await;

    let test_cases = vec![
        (
            serde_json::json!({
            "text_content": "Newsletter body as plain text",
            "html_content": "<p>Newsletter body as HTML</p>",
            }),
            "missing title",
        ),
        (
            serde_json::json!({"title": "Newsletter!", "text_content": "123"}),
            "missing html_content",
        ),
        (
            serde_json::json!({"title": "Newsletter!", "html_content": "123"}),
            "missing text_content",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = app.post_publish_newsletter(invalid_body).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {error_message}.",
        );
    }
}

/// Use the public API of the application under test to create an unconfirmed subscriber.
async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=john%20doe&email=the_john_doe%40example.com";
    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        // Returns a MockGuard which has drops at end of scope
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(body)
        .await
        .error_for_status()
        .unwrap();

    // inspect requests received by mock Postmark(email) server
    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    app.get_confirmation_links(email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_link = create_unconfirmed_subscriber(app).await;

    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

#[tokio::test]
async fn you_must_be_logged_in_to_see_the_newsletter_form() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app.get_publish_newsletter().await;

    // Assert
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn you_must_be_logged_in_to_publish_a_newsletter() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
        "idempotency_key": uuid::Uuid::new_v4().to_string()
    });
    let response = app.post_publish_newsletter(newsletter_request_body).await;

    // Assert
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn newsletter_creation_is_idempotent() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.test_user.login(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act - Part 1 - Submit newsletter form
    let newsletter_request_body = serde_json::json!({
    "title": "Newsletter title",
    "text_content": "Newsletter body as plain text",
    "html_content": "<p>Newsletter body as HTML</p>",
    // expect idempotency key as part of form data, not as an header
    "idempotency_key": uuid::Uuid::new_v4().to_string()
    });
    let response = app
        .post_publish_newsletter(newsletter_request_body.clone())
        .await;

    // Assert
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act - Part 2 - Follow the redirect
    let html_page = app.get_publish_newsletter_html().await;

    // Assert
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));

    // Act - Part 3 - Submit newsletter form **again**
    let response = app.post_publish_newsletter(newsletter_request_body).await;

    // Assert
    assert_is_redirect_to(&response, "/admin/newsletters");

    // Act - Part 4 - Follow the redirect
    let html_page = app.get_publish_newsletter_html().await;

    // Assert
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));

    // Mock verifies on Drop that we have sent the newsletter email **once**
}
