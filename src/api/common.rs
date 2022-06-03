use actix_web::HttpResponse;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
struct ErrorResponse {
    pub error_message: String,
}

pub fn error_response(err: String) -> HttpResponse {
    HttpResponse::UnprocessableEntity().json(ErrorResponse { error_message: err })
}
