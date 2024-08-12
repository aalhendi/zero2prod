use zero2prod::configuration::get_configuration;
use zero2prod::startup::Application;
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
    let application = Application::build(configuration).await?;
    application.run_until_stopped().await
}
