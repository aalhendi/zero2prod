use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    startup::{get_connection_pool, Application},
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

impl TestApp {
    pub async fn post_subscriptions(&self, body: &'static str) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{address}/subscriptions", address = &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
}

// This is done to fully decouple test suite from underlying implementation details.
// We test the exact same way a user would (black box testing) to avoid user-visible regression
pub async fn spawn_app() -> TestApp {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    Lazy::force(&TRACING);

    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        // Random DB name. This is for test isolation
        c.database.database_name = Uuid::new_v4().to_string();
        // Port 0 is special-cased at OS level. OS will scan for available port and bind app to it
        c.application.port = 0;
        c
    };

    // Create and migrate the database
    configure_database(&configuration.database).await;

    // No need to propagate errors, we're running tests.
    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application");
    // Get port before spawning the app.
    let address = format!("http://127.0.0.1:{port}", port = application.port());

    #[allow(clippy::let_underscore_future)]
    // Launch server as background task. We don't need the handle, so its discarded.
    let _ = tokio::spawn(application.run_until_stopped());
    // tokio::test spins up a new runtime at beginning of each test case & shuts it down at the end of each test case.
    // when a tokio runtime is shut down all tasks spawned on it are dropped.

    TestApp {
        address,
        // NOTE(aalhendi): No clean-up step. Logical DBs are not being deleted.
        // Postgres instance is only used for test purposes and can easily be restarted.
        db_pool: get_connection_pool(configuration.database),
    }
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
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
