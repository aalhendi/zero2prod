use actix_web::{web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;

use crate::{
    domain::SubscriberEmail, email_client::EmailClient, routes::subscriptions::error_chain_fmt,
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

#[tracing::instrument(name = "Publish newsletter", skip(body, pool, email_client))]
pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> Result<HttpResponse, PublishError> {
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
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl actix_web::ResponseError for PublishError {
    fn status_code(&self) -> reqwest::StatusCode {
        match self {
            PublishError::UnexpectedError(_) => reqwest::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
    // fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {}
}
