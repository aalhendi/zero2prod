use crate::{
    domain::{PasswordResetToken, SubscriberEmail},
    email_client::EmailClient,
    routes::subscriptions::error_chain_fmt,
    startup::ApplicationBaseUrl,
    utils::see_other,
};
use actix_web::{error::InternalError, web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::anyhow;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
}

#[tracing::instrument(
    skip(form, pool, email_client, base_url),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
    )]
pub async fn reset_password(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
    // Use Actix's InternalError to take an error and its cause
) -> Result<HttpResponse, InternalError<ForgotPasswordError>> {
    let user_email = match SubscriberEmail::parse(form.0.email) {
        Ok(parsed_email) => parsed_email,
        Err(e) => {
            return Err(InternalError::from_response(
                ForgotPasswordError::InvalidEmailError(anyhow!(e)),
                HttpResponse::BadRequest().finish(),
            ))
        }
    };

    let (user_id, username) = match get_user_id_and_username_by_email(&pool, &user_email).await {
        Ok(Some((id, username))) => (id, username),
        Ok(None) => {
            // Security: do not reveal that the user doesn't exist. Act as if eveything is fine. This prevents user enumeration.
            FlashMessage::info("If that email is in our system, we sent a reset link.").send();
            // TODO(aalhendi): Is this it? or do I want to redirect?
            return Ok(HttpResponse::Ok().finish());
        }
        Err(e) => {
            return Err(InternalError::from_response(
                ForgotPasswordError::UnexpectedError(anyhow!(e)),
                HttpResponse::InternalServerError().finish(),
            ))
        }
    };

    let reset_token = PasswordResetToken::default();

    if let Err(e) = insert_reset_token(&pool, user_id, &reset_token).await {
        return Err(InternalError::from_response(
            ForgotPasswordError::UnexpectedError(anyhow!(e)),
            HttpResponse::InternalServerError().finish(),
        ));
    }

    if let Err(e) = send_password_reset_email(
        &email_client,
        &username,
        &user_email,
        &base_url.0,
        &reset_token,
    )
    .await
    {
        return Err(InternalError::from_response(
            ForgotPasswordError::UnexpectedError(e.into()),
            HttpResponse::InternalServerError().finish(),
        ));
    }

    FlashMessage::info("If that email is in our system, we sent a reset link.").send();
    Ok(see_other("/login"))
}

#[tracing::instrument(
    name = "Send a password reset email to a user"
    skip(email_client, username, base_url, reset_token)
)]
/// Send a email to the new subscriber.
/// TODO(aalhendi) email delivery errors
pub async fn send_password_reset_email(
    email_client: &EmailClient,
    username: &String,
    email: &SubscriberEmail,
    base_url: &str,
    reset_token: &PasswordResetToken,
) -> Result<(), reqwest::Error> {
    // Build confirmation link with a dynamic root
    let reset_link = format!(
        "{base_url}/password-reset?token={reset_token}",
        reset_token = reset_token.as_ref()
    );
    let html_body = format!(
        "Dear {username},<br />\
                Click <a href=\"{reset_link}\">here</a> to reset your password.",
    );
    let plain_body = format!("Dear {username},\nVisit {reset_link} to reset your password.",);
    email_client
        .send_email(email, "Password Reset Request", &html_body, &plain_body)
        .await
}

#[tracing::instrument(name = "Lookup user by email", skip(email, pool))]
async fn get_user_id_and_username_by_email(
    pool: &PgPool,
    email: &SubscriberEmail,
) -> Result<Option<(Uuid, String)>, sqlx::Error> {
    let maybe_user = sqlx::query!(
        r#"
        SELECT user_id, username
        FROM users
        WHERE email = $1
        "#,
        email.as_ref()
    )
    .fetch_optional(pool)
    .await?;

    Ok(maybe_user.map(|r| (r.user_id, r.username)))
}

#[tracing::instrument(name = "Insert password reset token", skip(pool, user_id, reset_token))]
async fn insert_reset_token(
    pool: &PgPool,
    user_id: Uuid,
    reset_token: &PasswordResetToken,
) -> Result<(), sqlx::Error> {
    let token_hash = const_hex::encode(Sha256::digest(reset_token.as_ref()));
    sqlx::query!(
        r#"
        INSERT INTO password_resets (user_id, token_hash, created_at, expires_at)
        VALUES ($1, $2, NOW(), NOW() + INTERVAL '1 hour')
        "#,
        user_id,
        token_hash,
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(thiserror::Error)]
pub enum ForgotPasswordError {
    #[error("Invalid Email")]
    InvalidEmailError(#[source] anyhow::Error),
    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}
impl std::fmt::Debug for ForgotPasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
