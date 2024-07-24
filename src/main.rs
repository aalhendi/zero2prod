use sqlx::PgPool;
use std::net::TcpListener;
use zero2prod::configuration::get_configuration;
use zero2prod::startup;

// Used because entrypoint binary can't be an async fn.
// Because Rust does not come with Async Runtime by default, BYO.
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let configuration = get_configuration().expect("Failed to read configuration.");
    let connection_pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to conenct to Postgres.");
    let address = format!("127:0.0.1:{port}", port = configuration.application_port);
    let listener = TcpListener::bind(address).expect("Failed to bind to port 8000");
    startup::run(listener, connection_pool)?.await
}
