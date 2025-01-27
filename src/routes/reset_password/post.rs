use crate::{
    domain::{PasswordResetToken, SubscriberEmail},
    email_client::EmailClient,
    repository::{password_resets::PasswordResetRepository, user_repository::UserRepository},
    routes::subscriptions::error_chain_fmt,
    startup::ApplicationBaseUrl,
    utils::see_other,
};
use actix_web::{error::InternalError, web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::anyhow;
use sqlx::PgPool;

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

    let password_reset_repo = PasswordResetRepository::new(&pool);
    let user_repo = UserRepository::new(&pool);

    let (user_id, username) = match user_repo
        .get_user_id_and_username_by_email(&user_email)
        .await
    {
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

    if let Err(e) = password_reset_repo
        .insert_reset_token(user_id, &reset_token)
        .await
    {
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
