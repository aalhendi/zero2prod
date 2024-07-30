use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use tracing::Instrument;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

/*
Before calling subscribe() actix-web invokes the from_request method for all subscribe’s input arguments
Form::from_request tries to deserialise body into FormData via serde deserialize, according to URL encoding
if Form::from_request fails: returns 400 BAD REQUEST. If succeeds, subscribe() invoked and returns 200 OK
*/
pub async fn subscribe(
    form: web::Form<FormData>,
    // Retrieved from app state
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let request_id = Uuid::new_v4();
    // Associating structured info to our span as collection of KV pairs. `%` tells `tracing::` to use the `Display` implementation of the value.
    let request_span = tracing::info_span!("Adding a new subscriber.", %request_id, subscriber_email = %form.email, subscriber_name = %form.name);
    // NOTE(aalhendi): Careful what you log. i.e. PII must follow GDPR for EU services.
    tracing::info!(
        "request id {request_id} - Adding '{email}' '{name}' as a new subscriber.",
        email = form.email,
        name = form.name
    );

    let _request_span_guard = request_span.enter();

    let query_span = tracing::info_span!("Saving subscriber details in the database.");
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    // `get_ref` to get immutable ref to `PgConnection` wrapped by `web::Data`.
    .execute(pool.get_ref())
    // Instrument before await. This enters span every time its polled by executer and and exits it every time its parked to not mix up spans when juggling async tasks.
    .instrument(query_span)
    .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            tracing::error!("Failed to execute query: {e:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
