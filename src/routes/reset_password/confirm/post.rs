use crate::{
    authentication::{middleware::UserId, password::PasswordService},
    domain::{PasswordResetToken, SubscriberPassword},
    repository::password_resets::PasswordResetRepository,
    routes::subscriptions::error_chain_fmt,
    utils::see_other,
};
use actix_web::{http::StatusCode, web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

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

    let password_reset_repo = PasswordResetRepository::new(&pool);

    // 2. Check token validity
    let (user_id, token_hash) = password_reset_repo
        .get_user_id_by_token(&token)
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
    password_reset_repo
        .mark_token_as_used(&token_hash)
        .await
        .context("Failed to mark token as used")
        .map_err(ResetPasswordError::Unexpected)?;

    FlashMessage::success("Your password has been reset.").send();
    Ok(see_other("/login"))
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
