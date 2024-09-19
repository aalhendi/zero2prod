use zero2prod::configuration::get_configuration;
use zero2prod::issue_delivery_worker::run_worker_until_stopped;
use zero2prod::startup::Application;
use zero2prod::telemetry::{self};

// Used because entrypoint binary can't be an async fn.
// Because Rust does not come with Async Runtime by default, BYO.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let configuration = get_configuration().expect("Failed to read configuration.");

    let subscriber = telemetry::get_subscriber(
        String::from("zero2prod"),
        String::from("info"),
        std::io::stdout,
    );

    // guard will automatically call shutdown for OpenTelemetry when it goes out of scope
    #[cfg(feature = "open-telemetry")]
    let (subscriber, _guard) = telemetry::add_otel_to_subscriber(subscriber, &configuration.otel);

    telemetry::init_subscriber(subscriber);

    // Futures are lazy in Rust... Nothing is run until its awaited.
    let application = Application::build(configuration.clone()).await?;
    // We want concurrency AND parallelism, so we spawn async exprs and join the handles.
    let application_task = tokio::spawn(application.run_until_stopped());
    let worker_task = tokio::spawn(run_worker_until_stopped(configuration));

    // Run background worker and API side by ride.
    tokio::select! {
        o = application_task => report_exit("API", o),
        o = worker_task => report_exit("Background worker", o),
    }

    Ok(())
}

fn report_exit(
    task_name: &str,
    outcome: Result<Result<(), impl std::fmt::Debug + std::fmt::Display>, tokio::task::JoinError>,
) {
    match outcome {
        Ok(Ok(())) => {
            tracing::info!("{task_name} has exited")
        }

        Ok(Err(e)) => {
            tracing::error!(
            error.cause_chain = ?e,
            error.message = %e,
            "{task_name} failed",
            )
        }

        Err(e) => {
            tracing::error!(
            error.cause_chain = ?e,
            error.message = %e,
            "{task_name}' task failed to complete",
            )
        }
    }
}
