use std::net::TcpListener;
use zero2prod::configuration::get_configuration;
use zero2prod::run;

// Used because entrypoint binary can't be an async fn.
// Because Rust does not come with Async Runtime by default, BYO.
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let configuration = get_configuration().expect("Failed to read configuration.");
    let address = format!("127:0.0.1:{port}", port = configuration.application_port);
    let listener = TcpListener::bind(address).expect("Failed to bind to port 8000");
    run(listener)?.await
}
