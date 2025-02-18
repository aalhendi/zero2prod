use std::net::TcpListener;

use actix_session::{storage::RedisSessionStore, SessionMiddleware};
use actix_web::{cookie::Key, dev::Server, middleware::from_fn, web, App, HttpServer};
use actix_web_flash_messages::{storage::CookieMessageStore, FlashMessagesFramework};
use secrecy::{ExposeSecret, Secret, SecretString};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing_actix_web::TracingLogger;

use crate::{
    authentication::{self, password::PasswordService},
    configuration::{DatabaseSettings, Settings},
    email_client::EmailClient,
    routes,
};

#[derive(Debug, Clone)]
pub struct HmacSecret(Secret<String>);

impl HmacSecret {
    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }
}

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        let connection_pool = get_connection_pool(configuration.database);

        // Build an `EmailClient` using `configuration`
        let email_client = configuration.email_client.client();

        let address = format!(
            "{host}:{port}",
            host = configuration.application.host,
            port = configuration.application.port
        );
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(
            listener,
            connection_pool,
            email_client,
            configuration.application.base_url,
            HmacSecret(configuration.application.hmac_secret),
            configuration.redis_uri,
            configuration.auth.pepper,
        )
        .await?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// A more expressive name to make it clear that this function only returns when application is stopped.
    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn get_connection_pool(configuration: DatabaseSettings) -> PgPool {
    // Lazy: only try to establish connection when pool is used for the first time
    PgPoolOptions::new().connect_lazy_with(configuration.with_db())
}

/// Wrapper type in order to retrieve the URL
/// Since actix-web retrival from context is type-based, using raw `String` causes conflicts.
pub struct ApplicationBaseUrl(pub String);

async fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: HmacSecret,
    redis_uri: Secret<String>,
    pepper: SecretString,
) -> Result<Server, anyhow::Error> {
    // Wrap the connection in a smart pointer (Arc)
    let db_pool = web::Data::new(db_pool);
    // Wrap the email client in a smart pointer. Used rather than normal clone on EmailClient to avoid additional memory allocations.
    // reqwest::Client uses Arc<T> internally and does not need this. However, our additional fields do.
    let email_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));
    let secret_key = Key::from(hmac_secret.expose().as_bytes());
    let message_store = CookieMessageStore::builder(secret_key.clone()).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();
    let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;

    // Handles all *transport level* concerns
    /*
    HttpServer::new doesn't take App as arg - wants closure & returns App struct.
    due to actix-web’s runtime model: it will spin up worker process for each available core on machine.
    Each worker runs own copy of application built by HttpServer calling same closure HttpServer::new takes as argument.
    */
    let server = HttpServer::new(move || {
        // all app logic lives in App: routing, middlewares, request handlers, etc
        App::new()
            // Handles cookie hardening and does the heavy lifting for flash messages.
            .wrap(message_framework.clone())
            // Handles loading session data, tracking state changes + persisting them at end of request/response lifecycle
            .wrap(SessionMiddleware::new(
                redis_store.clone(),
                secret_key.clone(),
            ))
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
            .route(
                "/subscriptions/confirm",
                web::get().to(routes::subscriptions_confirm::confirm),
            )
            .route("/", web::get().to(routes::home::home))
            .route("/login", web::get().to(routes::login::get::login_form))
            .route("/login", web::post().to(routes::login::post::login))
            .route(
                "/password-reset",
                web::get().to(routes::reset_password::get::reset_password_form),
            )
            .route(
                "/password-reset",
                web::post().to(routes::reset_password::post::reset_password),
            )
            .route(
                "/password-reset/confirm",
                web::get().to(routes::reset_password::confirm::get::confirm_reset_password_form),
            )
            .route(
                "/password-reset/confirm",
                web::post().to(routes::reset_password::confirm::post::reset_password_confirm),
            )
            .service(
                web::scope("/admin")
                    .wrap(from_fn(authentication::middleware::reject_anonymous_users))
                    .route(
                        "/dashboard",
                        web::get().to(routes::admin::dashboard::admin_dashboard),
                    )
                    .route(
                        "/password",
                        web::get().to(routes::admin::password::get::change_password_form),
                    )
                    .route(
                        "/password",
                        web::post().to(routes::admin::password::post::change_password),
                    )
                    .route("/logout", web::post().to(routes::admin::logout::log_out))
                    .route(
                        "/newsletters",
                        web::get().to(routes::admin::newsletters::get::publish_newsletter_form),
                    )
                    .route(
                        "/newsletters",
                        web::post().to(routes::admin::newsletters::post::publish_newsletter),
                    ),
            )
            // Register the connection ptr copy as part of app state
            .app_data(db_pool.clone())
            // Client registered as part of app state to be able to reuse it across multiple requests.
            // reqwest::Client utilizes connection pooling to avoid socket exhaustion.
            // When cloning, we clone a pointer to the existing connection pool rather than making a new pool.
            .app_data(email_client.clone())
            .app_data(base_url.clone())
            .app_data(web::Data::new(hmac_secret.clone()))
            .app_data(web::Data::new(PasswordService::new(pepper.clone())))
    })
    .listen(listener)?
    .run();

    Ok(server)
}
