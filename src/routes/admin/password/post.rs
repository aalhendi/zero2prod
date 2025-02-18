use crate::{
    authentication::{
        middleware::UserId,
        password::{AuthError, Credentials, PasswordService},
    },
    domain::SubscriberPassword,
    repository::user_repository::UserRepository,
    utils::{e500, see_other},
};
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

pub async fn change_password(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
    password_service: web::Data<PasswordService>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();

    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        FlashMessage::error(
            "You entered two different new passwords - the field values must match.",
        )
        .send();
        return Ok(see_other("/admin/password"));
    }

    let user_repo = UserRepository::new(&pool);
    let username = user_repo.get_username(user_id).await.map_err(e500)?;
    let credentials = Credentials {
        username,
        password: form.0.current_password,
    };

    if let Err(e) = password_service
        .validate_credentials(credentials, &pool)
        .await
    {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("The current password is incorrect.").send();
                Ok(see_other("/admin/password"))
            }
            AuthError::UnexpectedError(_) => Err(e500(e)),
        };
    }

    let new_password = match SubscriberPassword::parse(form.0.new_password) {
        Ok(p) => p,
        Err(e) => {
            FlashMessage::error(e).send();
            return Ok(see_other("/admin/password"));
        }
    };

    password_service
        .change_password(user_id, new_password, &pool)
        .await
        .map_err(e500)?;
    FlashMessage::error("Your password has been changed.").send();
    Ok(see_other("/admin/password"))
}
