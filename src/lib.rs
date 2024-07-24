use std::net::TcpListener;

use actix_web::{dev::Server, web, App, HttpResponse, HttpServer};

pub mod configuration;
pub mod routes;
pub mod startup;

#[derive(serde::Deserialize)]
struct FormData {
    #[allow(dead_code)]
    email: String,
    #[allow(dead_code)]
    name: String,
}

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

/*
Before calling subscribe() actix-web invokes the from_request method for all subscribe’s input arguments
Form::from_request tries to deserialise body into FormData via serde deserialize, according to URL encoding
if Form::from_request fails: returns 400 BAD REQUEST. If succeeds, subscribe() invoked and returns 200 OK
*/
async fn subscribe(_form: web::Form<FormData>) -> HttpResponse {
    HttpResponse::Ok().finish()
}

pub fn run(listener: TcpListener) -> Result<Server, std::io::Error> {
    // Handles all *transport level* concerns
    let server = HttpServer::new(|| {
        // all app logic lives in App: routing, middlewares, request handlers, etc
        App::new()
            // short for Route::new().guard(guard::Get())
            // a.k.a. request should be passed to handler if and only if its HTTP method is GET.
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
    })
    .listen(listener)?
    .run();

    Ok(server)
}
