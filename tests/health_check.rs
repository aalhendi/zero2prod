use std::net::TcpListener;

use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    startup,
    telemetry::{get_subscriber, init_subscriber},
};

// TODO(aalhendi): Use some std:: methods. I don't think theres a need for the once_cell crate anymore.
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = String::from("info");
    let subscriber_name = String::from("test");
    // Can't assign subscriber to var and call `init_subscriber()` once becaue of opaque types. We can get around this with `Dyn` and `Box` but I'd rather not.
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    };
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

// This is done to fully decouple test suite from underlying implementation details.
// We test the exact same way a user would (black box testing) to avoid user-visible regression
async fn spawn_app() -> TestApp {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    Lazy::force(&TRACING);

    // No need to propagate errors, we're running tests.
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port");
    // Port 0 is special-cased at OS level. OS will scan for available port and bind app to it
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{port}");

    let mut configuration = get_configuration().expect("Failed to read configuration.");
    // Random DB name. This is for test isolation
    configuration.database.database_name = Uuid::new_v4().to_string();

    // NOTE(aalhendi): No clean-up step. Logical DBs are not being deleted.
    // Postgres instance is only used for test purposes and can easily be restarted.
    let connection_pool = configure_database(&configuration.database).await;

    let server = startup::run(listener, connection_pool.clone()).expect("Failed to bind address");
    // Launch server as background task. We don't need the handle, so its discarded.
    #[allow(clippy::let_underscore_future)]
    let _ = tokio::spawn(server);
    // tokio::test spins up a new runtime at beginning of each test case & shuts it down at the end of each test case.
    // when a tokio runtime is shut down all tasks spawned on it are dropped.

    TestApp {
        address,
        db_pool: connection_pool,
    }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create DB
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to conenct to Postgres.");

    connection
        .execute(
            format!(
                r#"CREATE DATABASE "{db_name}";"#,
                db_name = config.database_name
            )
            .as_str(),
        )
        .await
        .expect("Failed to create database.");

    // Migrate DB
    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database.");

    connection_pool
}

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(format!("{address}/health_check", address = app.address))
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
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    let body = "name=john%20doe&email=the_john_doe%40example.com";
    let response = client
        .post(&format!("{address}/subscriptions", address = app.address))
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
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=john%20doe", "missing the email"),
        ("email=the_john_doe%40example.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = client
            .post(&format!("{address}/subscriptions", address = app.address))
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

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=&email=the_john_doe%40example.com", "empty name"),
        ("name=John&email=", "empty email"),
        ("name=John&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        // Act
        let response = client
            .post(&format!("{address}/subscriptions", address = app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.");

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 200 OK when the payload was {description}."
        );
    }
}
