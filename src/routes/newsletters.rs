use crate::authentication::{validate_credentials, Credentials};
use actix_web::{http::header::HeaderMap, web, HttpResponse};
use anyhow::Context;
use base64::Engine;
use reqwest::header::HeaderValue;
use secrecy::Secret;
use sqlx::PgPool;

use crate::{
    authentication::AuthError, domain::SubscriberEmail, email_client::EmailClient,
    routes::subscriptions::error_chain_fmt,
};

// TODO(aalhendi): create a email domain entry?
#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}
#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    // header value, if present, must be valid UTF8 string
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header was missing")?
        .to_str()
        .context("The 'Authorization' header was not a valid UTF8 string.")?;
    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'.")?;
    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64encoded_segment)
        .context("Failed to base64-decode 'Basic' credentials.")?;
    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not valid UTF8.")?;
    // Split into two segments, using ':' as delimiter
    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth."))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth."))?
        .to_string();
    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(body, pool, email_client, request),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
    )]
pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    request: actix_web::HttpRequest,
) -> Result<HttpResponse, PublishError> {
    let credentials = basic_authentication(request.headers()).map_err(PublishError::AuthError)?;
    tracing::Span::current().record("username", tracing::field::display(&credentials.username));
    let user_id = validate_credentials(credentials, &pool)
        .await
        // Match on AuthError variants but pass **whole** error into PublishError constructors for context in middleware.
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => PublishError::AuthError(e.into()),
            AuthError::UnexpectedError(_) => PublishError::UnexpectedError(e.into()),
        })?;
    tracing::Span::current().record("user_id", tracing::field::display(&user_id));

    let subscribers = get_confirmed_subscribers(&pool).await?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    // format! allocates memory on heap to store output string
                    // `with_context` is lazy, so we dont allocate every time we send an email, only on error.
                    .with_context(|| {
                        format!(
                            "Failed to send newsletter issue to {email}",
                            email = subscriber.email
                        )
                    })?;
            }

            Err(error) => {
                tracing::warn!(
                // record error chain as structured field on log record.
                error.cause_chain = ?error,
                "Skipping a confirmed subscriber. Their stored contact details are invalid",
                );
            }
        }
    }

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let confirmed_subscribers = sqlx::query!(
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    // Re-validate emails. Why? No one-size-fits-all answer.
    // Data in DB creates temporal coupling between old and new versions of app.
    // So it might be valid when stored but current app version might disagree.
    // Since this is isn't critical, we will simply warn the operator.
    .map(|r| match SubscriberEmail::parse(r.email) {
        Ok(email) => Ok(ConfirmedSubscriber { email }),
        Err(error) => Err(anyhow::anyhow!(error)),
    })
    .collect();
    Ok(confirmed_subscribers)
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl actix_web::ResponseError for PublishError {
    // `status_code` is invoked by default `error_response` impl
    // we provide custom impl so no need to maintain `status_code` impl anymore.
    // fn status_code(&self) -> reqwest::StatusCode {}

    fn error_response(&self) -> HttpResponse {
        match self {
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(reqwest::StatusCode::INTERNAL_SERVER_ERROR)
            }

            PublishError::AuthError(_) => {
                let mut response = HttpResponse::new(reqwest::StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response
                    .headers_mut()
                    .insert(actix_web::http::header::WWW_AUTHENTICATE, header_value);
                response
            }
        }
    }
}
