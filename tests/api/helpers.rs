use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    email_client::EmailClient,
    issue_delivery_worker::{try_execute_task, ExecutionOutcome},
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

/// Confirmation links embedded in the request to the email API.
pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}
impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }
    async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        // Match parameters of the default password
        let password_hash = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();
        sqlx::query!(
            "INSERT INTO users (user_id, username, password_hash)
    VALUES ($1, $2, $3)",
            self.user_id,
            self.username,
            password_hash,
        )
        .execute(pool)
        .await
        .expect("Failed to store test user.");
    }

    pub async fn login(&self, app: &TestApp) {
        app.post_login(&serde_json::json!({
            "username": &self.username,
            "password": &self.password
        }))
        .await;
    }
}

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub email_server: wiremock::MockServer,
    pub test_user: TestUser,
    pub api_client: reqwest::Client,
    pub email_client: EmailClient,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        self.api_client
            .post(&format!("{address}/subscriptions", address = &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_publish_newsletter(&self, body: serde_json::Value) -> reqwest::Response {
        self.api_client
            .post(&format!(
                "{address}/admin/newsletters",
                address = &self.address
            ))
            .form(&body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_publish_newsletter(&self) -> reqwest::Response {
        self.api_client
            .get(&format!(
                "{address}/admin/newsletters",
                address = &self.address
            ))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_publish_newsletter_html(&self) -> String {
        self.get_publish_newsletter().await.text().await.unwrap()
    }

    /// Extract confirmation links embedded in a request to the email API.
    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        // Parse the body as JSON, starting from raw bytes
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
        // Extract the link from one of the request fields.
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str();
            let mut confirmation_link = reqwest::Url::parse(raw_link).unwrap();
            // Ensure no outside calls are made to random APIs on the web
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            // Non-issue for production workloads where the DNS domain is enough - just patch it in the test
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(body["TextBody"].as_str().unwrap());

        ConfirmationLinks { html, plain_text }
    }

    pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{address}/login", address = &self.address))
            // `reqwest` method ensures body is URL-encoded && `Content-Type` header is set accordingly.
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// Since tests will only look at HTML page, don't expose the underlying reqwest::Response
    pub async fn get_login_html(&self) -> String {
        self.api_client
            .get(&format!("{address}/login", address = &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
            .text()
            .await
            .unwrap()
    }

    pub async fn get_admin_dashboard(&self) -> reqwest::Response {
        self.api_client
            .get(&format!(
                "{address}/admin/dashboard",
                address = &self.address
            ))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_admin_dashboard_html(&self) -> String {
        self.get_admin_dashboard().await.text().await.unwrap()
    }

    pub async fn get_change_password(&self) -> reqwest::Response {
        self.api_client
            .get(&format!(
                "{address}/admin/password",
                address = &self.address
            ))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_change_password_html(&self) -> String {
        self.get_change_password().await.text().await.unwrap()
    }

    pub async fn post_change_password<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!(
                "{address}/admin/password",
                address = &self.address
            ))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_logout(&self) -> reqwest::Response {
        self.api_client
            .post(&format!("{address}/admin/logout", address = &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    // NOTE(aalhendi): This deviates slightly from main app functionality which spawns worker in background
    // This would require sleeping for arbitrary time intervals waiting for worker to process emails
    // and lead to fragile tests.
    /// Spawn worker on demaned, consume all enqueued tasks
    pub async fn dispatch_all_pending_emails(&self) {
        loop {
            if let ExecutionOutcome::EmptyQueue =
                try_execute_task(&self.db_pool, &self.email_client)
                    .await
                    .unwrap()
            {
                break;
            }
        }
    }
}

// This is done to fully decouple test suite from underlying implementation details.
// We test the exact same way a user would (black box testing) to avoid user-visible regression
pub async fn spawn_app() -> TestApp {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    Lazy::force(&TRACING);

    // Launch a mock server to stand in for Postmark's API
    let email_server = wiremock::MockServer::start().await;

    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        // Random DB name. This is for test isolation
        c.database.database_name = Uuid::new_v4().to_string();
        // Port 0 is special-cased at OS level. OS will scan for available port and bind app to it
        c.application.port = 0;
        // Use mock server as email API
        c.email_client.base_url = email_server.uri();
        c
    };

    // Create and migrate the database
    configure_database(&configuration.database).await;

    // No need to propagate errors, we're running tests.
    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application");
    let application_port = application.port();
    // Get port before spawning the app.
    let address = format!("http://127.0.0.1:{application_port}");

    #[allow(clippy::let_underscore_future)]
    // Launch server as background task. We don't need the handle, so its discarded.
    let _ = tokio::spawn(application.run_until_stopped());
    // tokio::test spins up a new runtime at beginning of each test case & shuts it down at the end of each test case.
    // when a tokio runtime is shut down all tasks spawned on it are dropped.

    let api_client = reqwest::Client::builder()
        // Prevent client from automatically handling redirect hops
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true) // Propagate cookies
        .build()
        .unwrap();

    let test_app = TestApp {
        address,
        port: application_port,
        // NOTE(aalhendi): No clean-up step. Logical DBs are not being deleted.
        // Postgres instance is only used for test purposes and can easily be restarted.
        db_pool: get_connection_pool(configuration.database),
        email_server,
        test_user: TestUser::generate(),
        api_client,
        email_client: configuration.email_client.client(),
    };
    test_app.test_user.store(&test_app.db_pool).await;
    test_app
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

pub fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), location);
}
