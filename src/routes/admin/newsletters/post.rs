use crate::authentication::middleware::UserId;
use crate::idempotency::key::IdempotencyKey;
use crate::idempotency::persistence::{save_response, try_processing, NextAction};
use crate::utils::{e400, e500, see_other};
use crate::{
    domain::SubscriberEmail, email_client::EmailClient, routes::subscriptions::error_chain_fmt,
};
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use reqwest::header::HeaderValue;
use sqlx::PgPool;

// TODO(aalhendi): create a email/newsletter domain entry?
#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    html_content: String,
    text_content: String,
    // include here since due to form submission, we do not have control over the headers being sent
    idempotency_key: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(form, pool, email_client, user_id),
    fields(user_id=%*user_id)
    )]
pub async fn publish_newsletter(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    // Destructure form to avoid making borrow checker angry
    let FormData {
        title,
        text_content,
        html_content,
        idempotency_key,
    } = form.0;
    let user_id = user_id.into_inner();
    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400)?;

    let transaction = match try_processing(&pool, &idempotency_key, user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(saved_response) => {
            // Return early if we have a saved response in the database
            success_message().send();
            return Ok(saved_response);
        }
    };

    let subscribers = get_confirmed_subscribers(&pool).await.map_err(e500)?;
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(&subscriber.email, &title, &html_content, &text_content)
                    .await
                    // format! allocates memory on heap to store output string
                    // `with_context` is lazy, so we dont allocate every time we send an email, only on error.
                    .with_context(|| {
                        format!(
                            "Failed to send newsletter issue to {email}",
                            email = subscriber.email
                        )
                    })
                    .map_err(e500)?;
            }

            Err(error) => {
                tracing::warn!(
                // record error chain as structured field on log record.
                error.cause_chain = ?error,
                error.message = %error,
                "Skipping a confirmed subscriber. Their stored contact details are invalid",
                );
            }
        }
    }

    success_message().send();
    let response = see_other("/admin/newsletters");
    let response = save_response(transaction, &idempotency_key, user_id, response)
        .await
        .map_err(e500)?;
    Ok(response)
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

fn success_message() -> FlashMessage {
    FlashMessage::info("The newsletter issue has been published!")
}
