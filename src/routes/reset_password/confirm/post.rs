use crate::{
    authentication::{middleware::UserId, password::PasswordService},
    domain::{PasswordResetToken, SubscriberPassword},
    routes::subscriptions::error_chain_fmt,
    utils::see_other,
};
use actix_web::{http::StatusCode, web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use secrecy::{ExposeSecret, Secret};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    token: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

pub async fn reset_password_confirm(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    password_service: web::Data<PasswordService>,
) -> Result<HttpResponse, actix_web::Error> {
    // 1. Validate token
    let token = PasswordResetToken::parse(form.0.token.expose_secret().to_string())
        .map_err(|_| ResetPasswordError::InvalidToken)?;

    // 2. Check token validity
    let (user_id, token_hash) = get_user_id_by_token(&pool, &token)
        .await
        .context("Failed to query password reset token")
        .map_err(ResetPasswordError::Unexpected)?
        .ok_or(ResetPasswordError::InvalidToken)?;

    // 3. Validate passwords match
    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        FlashMessage::error(
            "You entered two different new passwords - the field values must match.",
        )
        .send();
        return Ok(see_other(&format!(
            "/password-reset/confirm?token={token}",
            token = token.as_ref()
        )));
    }

    // 4. Validate password strength
    let new_password = match SubscriberPassword::parse(form.0.new_password) {
        Ok(p) => p,
        Err(e) => {
            FlashMessage::error(e).send();
            return Ok(see_other(&format!(
                "/password-reset/confirm?token={token}",
                token = token.as_ref()
            )));
        }
    };

    // 5. Update password
    password_service
        .change_password(UserId::from(user_id), new_password, &pool)
        .await
        .map_err(ResetPasswordError::Unexpected)?;

    // 6. Mark token as used instead of deleting
    mark_token_as_used(&pool, &token_hash)
        .await
        .context("Failed to mark token as used")
        .map_err(ResetPasswordError::Unexpected)?;

    FlashMessage::success("Your password has been reset.").send();
    Ok(see_other("/login"))
}

#[tracing::instrument(name = "Get user_id by password reset token", skip(token, pool))]
async fn get_user_id_by_token(
    pool: &PgPool,
    token: &PasswordResetToken,
) -> Result<Option<(Uuid, String)>, sqlx::Error> {
    let token_hash = const_hex::encode(Sha256::digest(token.as_ref()));
    sqlx::query!(
        r#"SELECT user_id, token_hash FROM password_resets 
           WHERE token_hash = $1 
           AND expires_at > NOW()
           AND used_at IS NULL"#,
        token_hash
    )
    .fetch_optional(pool)
    .await
    .map(|r| r.map(|row| (row.user_id, row.token_hash)))
}

#[tracing::instrument(name = "Mark password reset token as used", skip(pool, token_hash))]
async fn mark_token_as_used(pool: &PgPool, token_hash: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE password_resets 
        SET used_at = NOW() 
        WHERE token_hash = $1",
        token_hash
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(thiserror::Error)]
pub enum ResetPasswordError {
    #[error("Invalid or expired password reset token.")]
    InvalidToken,
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

// Use custom `Debug` impl for nice report using error source chain
impl std::fmt::Debug for ResetPasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl actix_web::ResponseError for ResetPasswordError {
    fn status_code(&self) -> StatusCode {
        match self {
            ResetPasswordError::InvalidToken => StatusCode::BAD_REQUEST,
            ResetPasswordError::Unexpected(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
    // fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {}
}
