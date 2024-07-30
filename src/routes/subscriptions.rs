use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

// Associating structured info to our span as collection of KV pairs. `%` tells `tracing::` to use the `Display` implementation of the value.
#[tracing::instrument(
name = "Adding a new subscriber",
skip(form, pool),
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
) -> HttpResponse {
    match insert_subscriber(&pool, &form).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(form, pool)
)]
async fn insert_subscriber(pool: &PgPool, form: &FormData) -> Result<(), sqlx::Error> {
    sqlx::query!(
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
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {e:?}");
        e
    })?;
    Ok(())
}
