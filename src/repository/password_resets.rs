use crate::domain::PasswordResetToken;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PasswordResetRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> PasswordResetRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    #[tracing::instrument(name = "Insert password reset token", skip(self, user_id, reset_token))]
    pub async fn insert_reset_token(
        &self,
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
        .execute(self.pool)
        .await?;
        Ok(())
    }

    #[tracing::instrument(name = "Get user_id by password reset token", skip(self, token))]
    pub async fn get_user_id_by_token(
        &self,
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
        .fetch_optional(self.pool)
        .await
        .map(|r| r.map(|row| (row.user_id, row.token_hash)))
    }

    #[tracing::instrument(name = "Mark password reset token as used", skip(self, token_hash))]
    pub async fn mark_token_as_used(&self, token_hash: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE password_resets 
            SET used_at = NOW() 
            WHERE token_hash = $1",
            token_hash
        )
        .execute(self.pool)
        .await?;
        Ok(())
    }
}
