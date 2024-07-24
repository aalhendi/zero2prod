use std::net::TcpListener;

use actix_web::{dev::Server, web, App, HttpServer};
use sqlx::PgPool;

use crate::routes;

pub fn run(listener: TcpListener, db_pool: PgPool) -> Result<Server, std::io::Error> {
    // Handles all *transport level* concerns
    /*
    HttpServer::new doesn't take App as arg - wants closure & returns App struct.
    due to actix-web’s runtime model: it will spin up worker process for each available core on machine.
    Each worker runs own copy of application built by HttpServer calling same closure HttpServer::new takes as argument.
    */
    // Wrap the connection in a smart pointer (Arc)
    let db_pool = web::Data::new(db_pool);
    let server = HttpServer::new(move || {
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
            // Register the connection ptr copy as part of app state
            .app_data(db_pool.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
