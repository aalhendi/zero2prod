use actix_web::{http::header::ContentType, web, HttpResponse};
use hmac::{Hmac, Mac};

use crate::startup::HmacSecret;

#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: String,
    tag: String,
}

impl QueryParams {
    fn verify(self, secret: &HmacSecret) -> Result<String, anyhow::Error> {
        let tag = hex::decode(self.tag)?;
        let query_string = format!("error={err}", err = urlencoding::Encoded::new(&self.error));
        let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret.expose().as_bytes()).unwrap();
        mac.update(query_string.as_bytes());
        mac.verify_slice(&tag)?;
        Ok(self.error)
    }
}

pub async fn login_form(
    query: Option<web::Query<QueryParams>>,
    secret: web::Data<HmacSecret>,
) -> HttpResponse {
    let error_html = match query {
        None => String::new(),
        Some(query) => match query.0.verify(&secret) {
            Ok(error) => {
                // Follow OWASP guidelines for untrusted input. HTML-encode html to prevent XSS.
                let formatted_msg = htmlescape::encode_minimal(&error);
                format!("<p><i>{formatted_msg}</i></p>")
            }
            Err(e) => {
                tracing::warn!(
                error.message = %e,
                error.cause_chain = ?e,
                "Failed to verify query parameters using the HMAC tag"
                );
                String::new()
            }
        },
    };

    // TODO(aalhendi): what to do with login.html? include str and str replace?
    let html_body = format!(
        r#"
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta http-equiv="content-type" content="text/html; charset=utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Home</title>
  </head>
  <body>
    {error_html}
    <form action="/login" method="post">
      <label
        >Username
        <input type="text" placeholder="Enter Username" name="username" />
      </label>
      <label
        >Password
        <input type="password" placeholder="Enter Password" name="password" />
      </label>
      <button type="submit">Login</button>
    </form>
  </body>
</html>
        "#
    );

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html_body)
}
