use actix_web::{http::header::ContentType, web, HttpResponse};

#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: Option<String>,
}

pub async fn login_form(query: web::Query<QueryParams>) -> HttpResponse {
    let error_html = match query.0.error {
        None => String::new(),
        Some(error_message) => {
            // Follow OWASP guidelines for untrusted input. HTML-encode html to prevent XSS.
            let formatted_msg = htmlescape::encode_minimal(&error_message);
            format!("<p><i>{formatted_msg}</i></p>")
        }
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
