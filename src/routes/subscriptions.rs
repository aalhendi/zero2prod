use actix_web::{http::StatusCode, web, HttpResponse};
use anyhow::Context;
use chrono::Utc;
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    domain::{NewSubscriber, SubscriberEmail, SubscriberName, SubscriptionToken},
    email_client::EmailClient,
    startup::ApplicationBaseUrl,
};

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;
    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { email, name })
    }
}

// Associating structured info to our span as collection of KV pairs. `%` tells `tracing::` to use the `Display` implementation of the value.
#[tracing::instrument(
name = "Adding a new subscriber",
skip(form, pool, email_client, base_url),
fields(
// request_id comes from TracingLogger middleware
subscriber_email = %form.email,
subscriber_name = %form.name
)
)]
/*
Before calling subscribe() actix-web invokes the from_request method for all subscribe’s input arguments
Form::from_request tries to deserialise body into FormData via serde deserialize, according to URL encoding
if Form::from_request fails: returns 400 BAD REQUEST. If succeeds, subscribe() invoked and returns 200 OK
*/
pub async fn subscribe(
    form: web::Form<FormData>,
    // Retrieved from app state
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    // `web::Form` is a wrapper around `FormData`
    // `form.0` gives us access to the underlying `FormData`
    // `?` operator transparently invokes the `Into` trait
    let new_subscriber = form.0.try_into().map_err(SubscribeError::ValidationError)?;

    let mut transaction = pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;
    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .context("Failed to insert new subscriber in the database.")?;
    let subscription_token = SubscriptionToken::default();

    store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .context("Failed to store the confirmation token for a new subscriber.")?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new subscriber.")?;

    send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url.0,
        &subscription_token,
    )
    .await
    .context("Failed to send a confirmation email.")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, transaction)
)]
async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();

    let query = sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    );

    transaction.execute(query).await?;

    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, transaction)
)]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &SubscriptionToken,
) -> Result<(), StoreTokenError> {
    let query = sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
    VALUES ($1, $2)"#,
        subscription_token.as_ref(),
        subscriber_id
    );

    // Wrap underlying error
    transaction.execute(query).await.map_err(StoreTokenError)?;
    Ok(())
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber"
    skip(email_client, new_subscriber, base_url, subscription_token)
)]
/// Send a email to the new subscriber.
/// TODO(aalhendi) email delivery errors
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &SubscriptionToken,
) -> Result<(), reqwest::Error> {
    // Build confirmation link with a dynamic root
    let confirmation_link = format!(
        "{base_url}/subscriptions/confirm?subscription_token={subscription_token}",
        subscription_token = subscription_token.as_ref()
    );
    let html_body = format!(
        "Welcome to our newsletter!<br />\
                Click <a href=\"{confirmation_link}\">here</a> to confirm your subscription.",
    );
    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {confirmation_link} to confirm your subscription.",
    );
    email_client
        .send_email(&new_subscriber.email, "Welcome!", &html_body, &plain_body)
        .await
}

pub struct StoreTokenError(sqlx::Error);

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while trying to store a subscription token."
        )
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // Compiler transparently casts `&sqlx::Error` into `&dyn Error`
        Some(&self.0)
    }
}

// TODO(aalhendi): #![feature(error_iter)]
// TODO(aalhendi): move to utils?
// err.sources()
pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{e}\n")?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{cause}")?;
        current = cause.source();
    }
    Ok(())
}

// Introduced to enforce proper separation of concerns
// will own the HTTP-related logic (ResponseError’s implementation)
#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    // String does not implement `Error` - we consider it the root cause
    ValidationError(String),
    #[error(transparent)]
    // anyhow::Context exposes `.context()` which converts error to anyhow::Error
    // and allows additional context around caller intentions
    UnexpectedError(#[from] anyhow::Error),
}

// Use custom `Debug` impl for nice report using error source chain
impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl actix_web::ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
        match self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
    // fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {}
}
