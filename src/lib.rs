use std::sync::Once;

pub mod authentication;
pub mod configuration;
pub mod domain;
pub mod email_client;
pub mod idempotency;
pub mod issue_delivery_worker;
pub mod repository;
pub mod routes;
pub mod session_state;
pub mod startup;
pub mod telemetry;
pub mod utils;

pub(crate) fn install_ring_crypto_provider() {
    static INSTALL_CRYPTO_PROVIDER: Once = Once::new();

    INSTALL_CRYPTO_PROVIDER.call_once(|| {
        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("Failed to install the Ring crypto provider");
    });
}
