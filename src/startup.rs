use std::net::TcpListener;

use actix_web::{dev::Server, web, App, HttpServer};
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use crate::{email_client::EmailClient, routes};

pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    // Handles all *transport level* concerns
    /*
    HttpServer::new doesn't take App as arg - wants closure & returns App struct.
    due to actix-web’s runtime model: it will spin up worker process for each available core on machine.
    Each worker runs own copy of application built by HttpServer calling same closure HttpServer::new takes as argument.
    */
    // Wrap the connection in a smart pointer (Arc)
    let db_pool = web::Data::new(db_pool);

    // Wrap the email client in a smart pointer. Used rather than normal clone on EmailClient to avoid additional memory allocations.
    // reqwest::Client uses Arc<T> internally and does not need this. However, our additional fields do.
    let email_client = web::Data::new(email_client);
    let server = HttpServer::new(move || {
        // all app logic lives in App: routing, middlewares, request handlers, etc
        App::new()
            // Using drop in replacement for actix::middleware::Logger that knows how to handle the tracing crate (tracing-aware)
            .wrap(TracingLogger::default())
            // short for Route::new().guard(guard::Get())
            // a.k.a. request should be passed to handler if and only if its HTTP method is GET.
            .route(
                "/health_check",
                web::get().to(routes::health_check::health_check),
            )
            .route(
                "/subscriptions",
                web::post().to(routes::subscriptions::subscribe),
            )
            // Register the connection ptr copy as part of app state
            .app_data(db_pool.clone())
            // Client registered as part of app state to be able to reuse it across multiple requests.
            // reqwest::Client utilizes connection pooling to avoid socket exhaustion.
            // When cloning, we clone a pointer to the existing connection pool rather than making a new pool.
            .app_data(email_client.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
