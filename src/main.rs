use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use zero2prod::configuration::get_configuration;
use zero2prod::email_client::EmailClient;
use zero2prod::startup;
use zero2prod::telemetry;

// Used because entrypoint binary can't be an async fn.
// Because Rust does not come with Async Runtime by default, BYO.
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = telemetry::get_subscriber(
        String::from("zero2prod"),
        String::from("info"),
        std::io::stdout,
    );
    telemetry::init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    let connection_pool =
        // Lazy: only try to establish connection when pool is used for the first time
        PgPoolOptions::new().connect_lazy_with(configuration.database.with_db());

    // Build an `EmailClient` using `configuration`
    let sender_email = configuration
        .email_client
        .sender()
        .expect("Invalid sender email address.");
    let base_url = reqwest::Url::parse(&configuration.email_client.base_url)
        .expect("Failed to parse base URL.");
    let email_client = EmailClient::new(
        base_url,
        sender_email,
        configuration.email_client.authorization_token,
    );

    let address = format!(
        "{host}:{port}",
        host = configuration.application.host,
        port = configuration.application.port
    );
    let listener = TcpListener::bind(address).expect("Failed to bind to port 8000");
    startup::run(listener, connection_pool, email_client)?.await
}
