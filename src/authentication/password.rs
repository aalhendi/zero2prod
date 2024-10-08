use std::sync::Arc;

use super::middleware::UserId;
use crate::{domain::SubscriberPassword, telemetry::spawn_blocking_with_tracing};
use anyhow::Context;
use argon2::{
    password_hash::SaltString, Algorithm, Argon2, Params, PasswordHash, PasswordHasher,
    PasswordVerifier, Version,
};
use secrecy::{ExposeSecret, Secret, SecretString};
use sqlx::PgPool;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid Credentials.")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

pub struct PasswordService {
    pepper: Arc<SecretString>,
}

impl PasswordService {
    pub fn new(pepper: SecretString) -> Self {
        Self {
            pepper: Arc::new(pepper),
        }
    }

    #[tracing::instrument(name = "Validate credentials", skip(self, credentials, pool))]
    pub async fn validate_credentials(
        &self,
        credentials: Credentials,
        pool: &PgPool,
    ) -> Result<uuid::Uuid, AuthError> {
        let mut user_id = None;
        // Establish fallback password (with salt and load parameters)
        // to perform same amount of work whether user exists or doesn't
        let mut expected_password_hash = Secret::new(
            "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
                .to_string(),
        );

        if let Some((stored_user_id, stored_password_hash)) = self
            .get_stored_credentials(&credentials.username, pool)
            .await?
        {
            user_id = Some(stored_user_id);
            expected_password_hash = stored_password_hash;
        }

        // Offload CPU-intensive workload (>1ms) to sperate threadpool.
        // These are reserved for blocking ops and don't interfere with scheduling of async tasks.
        let pepper = self.pepper.clone();
        spawn_blocking_with_tracing(move || {
            PasswordService::verify_password_hash(
                expected_password_hash,
                credentials.password,
                pepper,
            )
        })
        .await
        .context("Failed to spawn blocking task.")??;

        // Only set to `Some` if found credentials in store.
        // Even if fallback password ends up matching, never authenticate non-existing user.
        // TODO(aalhendi): add unit test for fallback pw
        user_id
            .ok_or_else(|| anyhow::anyhow!("Unknown username."))
            .map_err(AuthError::InvalidCredentials)
    }

    #[tracing::instrument(
        name = "Verify password hash",
        skip(expected_password_hash, password_candidate, pepper)
    )]
    fn verify_password_hash(
        expected_password_hash: Secret<String>,
        password_candidate: Secret<String>,
        pepper: Arc<SecretString>,
    ) -> Result<(), AuthError> {
        let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
            .context("Failed to parse hash in PHC string format.")?;

        let mut peppered_password = password_candidate.expose_secret().as_bytes().to_vec();
        peppered_password.extend_from_slice(pepper.expose_secret().as_bytes());

        Argon2::default()
            .verify_password(&peppered_password, &expected_password_hash)
            .context("Invalid password.")
            .map_err(AuthError::InvalidCredentials)
    }

    #[tracing::instrument(name = "Get stored credentials", skip(username, pool, self))]
    async fn get_stored_credentials(
        &self,
        username: &str,
        pool: &PgPool,
    ) -> Result<Option<(uuid::Uuid, Secret<String>)>, anyhow::Error> {
        let row = sqlx::query!(
            r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1
        "#,
            username,
        )
        .fetch_optional(pool)
        .await
        .context("Failed to perform a query to retrieve stored credentials.")?
        .map(|row| (row.user_id, Secret::new(row.password_hash)));

        Ok(row)
    }

    #[tracing::instrument(name = "Change password", skip(password, pool, self))]
    pub async fn change_password(
        &self,
        user_id: UserId,
        password: SubscriberPassword,
        pool: &PgPool,
    ) -> Result<(), anyhow::Error> {
        let pepper = self.pepper.clone();
        let password_hash = spawn_blocking_with_tracing(move || {
            PasswordService::compute_password_hash(password, pepper)
        })
        .await?
        .context("Failed to hash password")?;
        sqlx::query!(
            r#"
UPDATE users
SET password_hash = $1
WHERE user_id = $2
"#,
            password_hash.expose_secret(),
            *user_id
        )
        .execute(pool)
        .await
        .context("Failed to change user's password in the database.")?;
        Ok(())
    }

    fn compute_password_hash(
        password: SubscriberPassword,
        pepper: Arc<SecretString>,
    ) -> Result<Secret<String>, anyhow::Error> {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let mut peppered_password = password.expose().as_bytes().to_vec();
        peppered_password.extend_from_slice(pepper.expose_secret().as_bytes());

        let password_hash = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(&peppered_password, &salt)?
        .to_string();
        Ok(Secret::new(password_hash))
    }
}
