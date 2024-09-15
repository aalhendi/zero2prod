use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::SubscriptionToken;
use crate::routes::subscriptions::error_chain_fmt;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters))]
pub async fn confirm(
    parameters: web::Query<Parameters>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, ConfirmError> {
    // Fn param of type web::Query<Parameter> to confirm is enough to instruct actix-web to only call handler if extraction was successful.
    // If extraction failed, 400 Bad Request is automatically returned to the caller.

    let subscription_token = SubscriptionToken::parse(parameters.subscription_token.clone())
        .map_err(ConfirmError::ValidationError)?;

    let subscriber_id = get_subscriber_id_from_token(&pool, &subscription_token)
        .await
        .context("Failed to fetch subscriber id from token.")?
        .ok_or(ConfirmError::UnknownToken)?;

    confirm_subscriber(&pool, subscriber_id)
        .await
        .context("Failed to update subscriber status to `confirmed`.")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, pool))]
pub async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id,
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(subscription_token, pool))]
pub async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &SubscriptionToken,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT subscriber_id FROM subscription_tokens \
    WHERE subscription_token = $1",
        subscription_token.as_ref(),
    )
    .fetch_optional(pool)
    .await?;
    Ok(result.map(|r| r.subscriber_id))
}

#[derive(thiserror::Error)]
pub enum ConfirmError {
    #[error("No subscriber associated with the provided token.")]
    UnknownToken,
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

// Use custom `Debug` impl for nice report using error source chain
impl std::fmt::Debug for ConfirmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl actix_web::ResponseError for ConfirmError {
    fn status_code(&self) -> StatusCode {
        match self {
            ConfirmError::UnknownToken => StatusCode::UNAUTHORIZED,
            ConfirmError::ValidationError(_) => StatusCode::BAD_REQUEST,
            ConfirmError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
    // fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {}
}
