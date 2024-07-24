use actix_web::{dev::Server, web, App, HttpServer};
use std::net::TcpListener;

pub mod configuration;
pub mod routes;
pub mod startup;

pub fn run(listener: TcpListener) -> Result<Server, std::io::Error> {
    // Handles all *transport level* concerns
    let server = HttpServer::new(|| {
        // all app logic lives in App: routing, middlewares, request handlers, etc
        App::new()
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
    })
    .listen(listener)?
    .run();

    Ok(server)
}
