use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

#[derive(serde::Deserialize)]
pub struct Parameters {
    reset_token: String,
}

#[tracing::instrument(name = "Confirm reset password request", skip(parameters))]
pub async fn confirm_reset_password_form(
    flash_messages: IncomingFlashMessages,
    parameters: web::Query<Parameters>,
) -> HttpResponse {
    // Fn param of type web::Query<Parameter> to confirm is enough to instruct actix-web to only call handler if extraction was successful.
    // If extraction failed, 400 Bad Request is automatically returned to the caller.

    let reset_token = PasswordResetToken::parse(parameters.subscription_token.clone())
        .map_err(ConfirmError::ValidationError)?;

    let mut msg_html = String::new();
    for m in flash_messages.iter() {
        writeln!(msg_html, "<p><i>{content}</i></p>", content = m.content()).unwrap();
    }
    // TODO(aalhendi): Move to file + components
    let html_body = format!(
        r#"
        r#"<!DOCTYPE html>
        <html lang="en">
          <head>
            <meta charset="UTF-8" />
            <meta http-equiv="content-type" content="text/html; charset=utf-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1.0" />
            <title>Change Password</title>
          </head>
          <body>
            {msg_html}
            <form action="/admin/password" method="post">
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
              <button type="submit">Change password</button>
            </form>
            <p><a href="/admin/dashboard">&lt;- Back</a></p>
          </body>
        </html>"#,
    );

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html_body)
}
