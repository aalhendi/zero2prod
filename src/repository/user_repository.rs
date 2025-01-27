use crate::{authentication::middleware::UserId, domain::SubscriberEmail};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

pub struct UserRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> UserRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    #[tracing::instrument(name = "Get user_id and username by email", skip(self, email))]
    pub async fn get_user_id_and_username_by_email(
        &self,
        email: &SubscriberEmail,
        // TODO(aalhendi): UserId type?
    ) -> Result<Option<(Uuid, String)>, sqlx::Error> {
        let maybe_user = sqlx::query!(
            r#"
            SELECT user_id, username
            FROM users
            WHERE email = $1
            "#,
            email.as_ref()
        )
        .fetch_optional(self.pool)
        .await?;

        Ok(maybe_user.map(|r| (r.user_id, r.username)))
    }

    #[tracing::instrument(name = "Get username from user_id", skip(self))]
    // TODO(aalhendi): Subscriber Username type?
    pub async fn get_username(&self, user_id: UserId) -> Result<String, anyhow::Error> {
        let row = sqlx::query!(
            r#"
            SELECT username
            FROM users
            WHERE user_id = $1
            "#,
            *user_id,
        )
        .fetch_one(self.pool)
        .await
        .context("Failed to perform a query to retrieve a username.")?;
        Ok(row.username)
    }
}
