use actix_web::{http::header::ContentType, web, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

use crate::{domain::PasswordResetToken, routes::subscriptions_confirm::ConfirmError};

#[derive(serde::Deserialize)]
pub struct Parameters {
    token: String,
}

#[tracing::instrument(
    name = "Confirm reset password request",
    skip(parameters, flash_messages)
)]
pub async fn confirm_reset_password_form(
    flash_messages: IncomingFlashMessages,
    parameters: web::Query<Parameters>,
) -> Result<HttpResponse, actix_web::Error> {
    // Fn param of type web::Query<Parameter> to confirm is enough to instruct actix-web to only call handler if extraction was successful.
    // If extraction failed, 400 Bad Request is automatically returned to the caller.

    let password_reset_token = PasswordResetToken::parse(parameters.token.clone())
        .map_err(ConfirmError::ValidationError)?;

    // TODO(aalhendi): Do I even want to render this page if the token doesn't exist on DB? (invalid token)

    let mut msg_html = String::new();
    for m in flash_messages.iter() {
        writeln!(msg_html, "<p><i>{content}</i></p>", content = m.content()).unwrap();
    }
    // TODO(aalhendi): Move to file + components
    let html_body = format!(
        r#"
        <!DOCTYPE html>
        <html lang="en">
          <head>
            <meta charset="UTF-8" />
            <meta http-equiv="content-type" content="text/html; charset=utf-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1.0" />
            <title>Change Password</title>
          </head>
          <body>
            {msg_html}
            <form action="/password-reset/confirm" method="post">
              <label
                >Current password
                <input
                  type="password"
                  placeholder="Enter current password"
                  name="current_password"
                />
              </label>
              <br />
              <label
                >New password
                <input
                  type="password"
                  placeholder="Enter new password"
                  name="new_password"
                />
              </label>
              <br />
              <label
                >Confirm new password
                <input
                  type="password"
                  placeholder="Type the new password again"
                  name="new_password_check"
                />
              </label>
              <br />
                <input
                  type="hidden"
                  name="token"
                  value={token}
                />
              <button type="submit">Change password</button>
            </form>
            <p><a href="/login/">&lt;- Back</a></p>
          </body>
        </html>"#,
        token = password_reset_token.as_ref()
    );

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html_body))
}
