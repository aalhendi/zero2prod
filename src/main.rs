use std::net::TcpListener;

use zero2prod::run;

// Used because entrypoint binary can't be an async fn.
// Because Rust does not come with Async Runtime by default, BYO.
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let listener = TcpListener::bind("127:0.0.1:8000").expect("Failed to bind to port 8000");
    run(listener)?.await
}
