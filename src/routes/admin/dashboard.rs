use crate::{
    authentication::middleware::UserId, repository::user_repository::UserRepository, utils::e500,
};
use actix_web::{http::header::ContentType, web, HttpResponse};
use sqlx::PgPool;

pub async fn admin_dashboard(
    user_id: web::ReqData<UserId>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_repo = UserRepository::new(&pool);
    let user_id = user_id.into_inner();
    let username = user_repo.get_username(user_id).await.map_err(e500)?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta http-equiv="content-type" content="text/html; charset=utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Admin dashboard</title>
</head>
<body>
  <p>Welcome {username}!</p>
  <p>Available actions:</p>
  <ol>
    <li><a href="/admin/newsletters">Send a newsletter</a></li>
    <li><a href="/admin/password">Change password</a></li>
    <li>
      <form name="logoutForm" action="/admin/logout" method="post">
        <input type="submit" value="Logout">
      </form>
    </li>
  </ol>
</body>
</html>"#
        )))
}
