use actix_web::{get, HttpResponse};

pub mod rationale;

#[get("/")]
pub async fn index() -> HttpResponse {
    HttpResponse::Ok().body("seedwing-policy API server")
}
