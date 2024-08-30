use std::net::TcpListener;

use actix_session::{storage::RedisSessionStore, SessionMiddleware};
use actix_web::{cookie::Key, dev::Server, web, App, HttpServer};
use actix_web_flash_messages::{storage::CookieMessageStore, FlashMessagesFramework};
use secrecy::{ExposeSecret, Secret};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing_actix_web::TracingLogger;

use crate::{
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
        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address.");
        let base_url = reqwest::Url::parse(&configuration.email_client.base_url)
            .expect("Failed to parse base URL.");
        let timeout = configuration.email_client.timeout();
        let email_client = EmailClient::new(
            base_url,
            sender_email,
            configuration.email_client.authorization_token,
            timeout,
        );

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
) -> Result<Server, anyhow::Error> {
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
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));
    let secret_key = Key::from(hmac_secret.expose().as_bytes());
    let message_store = CookieMessageStore::builder(secret_key.clone()).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();
    let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;
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
            .route(
                "/newsletters",
                web::post().to(routes::newsletters::publish_newsletter),
            )
            .route("/", web::get().to(routes::home::home))
            .route("/login", web::get().to(routes::login::get::login_form))
            .route("/login", web::post().to(routes::login::post::login))
            .route(
                "/admin/dashboard",
                web::get().to(routes::admin::dashboard::admin_dashboard),
            )
            // Register the connection ptr copy as part of app state
            .app_data(db_pool.clone())
            // Client registered as part of app state to be able to reuse it across multiple requests.
            // reqwest::Client utilizes connection pooling to avoid socket exhaustion.
            // When cloning, we clone a pointer to the existing connection pool rather than making a new pool.
            .app_data(email_client.clone())
            .app_data(base_url.clone())
            .app_data(web::Data::new(hmac_secret.clone()))
    })
    .listen(listener)?
    .run();

    Ok(server)
}
