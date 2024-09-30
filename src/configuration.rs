use std::collections::HashMap;

use secrecy::{ExposeSecret, Secret};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use sqlx::ConnectOptions;

use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;

#[derive(serde::Deserialize, Clone)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    pub email_client: EmailClientSettings,
    // URI marked as secret because it may embed a password
    pub redis_uri: Secret<String>,
    pub otel: OpenTelemetrySettings,
    pub auth: AuthSettings,
}

#[derive(serde::Deserialize, Clone)]
pub struct AuthSettings {
    pub pepper: Secret<String>,
}

#[derive(serde::Deserialize, Clone)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
    pub authorization_token: Secret<String>,
    timeout_milliseconds: u64,
}

impl EmailClientSettings {
    pub fn sender(&self) -> Result<SubscriberEmail, String> {
        SubscriberEmail::parse(self.sender_email.clone())
    }

    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.timeout_milliseconds)
    }

    pub fn client(self) -> EmailClient {
        let sender_email = self.sender().expect("Invalid sender email address.");
        let timeout = self.timeout();
        let base_url =
            reqwest::Url::parse(&self.base_url).expect("Failed to parse email client base URL.");
        EmailClient::new(base_url, sender_email, self.authorization_token, timeout)
    }
}

#[derive(serde::Deserialize, Clone)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub base_url: String,
    pub hmac_secret: Secret<String>,
}

#[derive(serde::Deserialize, Clone)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}

impl DatabaseSettings {
    pub fn without_db(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            // Try an encrypted connection, fallback to unencrypted if it fails
            PgSslMode::Prefer
        };

        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mode)
    }

    pub fn with_db(&self) -> PgConnectOptions {
        self.without_db()
            .database(&self.database_name)
            .log_statements(tracing_log::log::LevelFilter::Trace)
    }
}

#[derive(serde::Deserialize, Clone)]
pub struct OpenTelemetrySettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    port: u16,
    base_url: String,
    trace_endpoint: String,
    log_endpoint: String,
    pub auth_token: Secret<String>,
}

impl OpenTelemetrySettings {
    pub fn trace_full_url(&self) -> String {
        format!(
            "{base_url}:{port}{endpoint}",
            base_url = self.base_url,
            port = self.port,
            endpoint = self.trace_endpoint
        )
    }

    pub fn log_full_url(&self) -> String {
        format!(
            "{base_url}:{port}{endpoint}",
            base_url = self.base_url,
            port = self.port,
            endpoint = self.log_endpoint
        )
    }

    pub fn headers(&self) -> HashMap<String, String> {
        HashMap::from([(
            String::from("authorization"),
            format!("Basic {token}", token = self.auth_token.expose_secret()),
        )])
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine the current directory.");
    let configutation_directory = base_path.join("configuration");

    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| String::from("local"))
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT");
    let environment_filename = format!("{environment}.yaml", environment = environment.as_str());

    let settings = config::Config::builder()
        .add_source(config::File::from(
            configutation_directory.join("base.yaml"),
        ))
        .add_source(config::File::from(
            configutation_directory.join(environment_filename),
        ))
        // Settings from env vars prefix `APP_<x>` E.g. `APP_APPLICATION__PORT=5001 sets `Settings.application.port`
        // Allows overriding whatever is in configuration file
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;

    settings.try_deserialize::<Settings>()
}

/// The possible runtime environment for our application.
pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{other} is not a supported environment. Use either `local` or `production`.",
            )),
        }
    }
}
