use actix_web::{post, web, HttpResponse, Responder};

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct FormData {
    email: String,
    name: String,
}

#[post("/subscriptions")]
async fn subscribe(_form: web::Form<FormData>) -> impl Responder {
    HttpResponse::Ok().finish()
}
