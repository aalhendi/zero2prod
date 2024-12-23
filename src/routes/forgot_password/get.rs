use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

pub async fn forgot_password_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    let mut error_html = String::new();
    for m in flash_messages.iter() {
        writeln!(error_html, "<p><i>{content}</i></p>", content = m.content()).unwrap();
    }
    // TODO(aalhendi): Move to file?
    let html_body = format!(
        r#"
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta http-equiv="content-type" content="text/html; charset=utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Forgot Password</title>
  </head>
  <body>
    {error_html}
    <form action="/password-reset" method="post">
      <label
        >Email
        <input type="email" placeholder="Enter Email" name="email" />
      </label>
      <button type="submit">Forgot Password</button>
    </form>
  </body>
</html>
        "#
    );

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html_body)
}
