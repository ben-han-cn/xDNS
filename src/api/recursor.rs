use actix_web::{error, middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use r53::{Name, RRset};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

use crate::recursor::Recursor;

#[derive(Serialize, Deserialize, Debug, Default)]
struct AddForwardRequest {
    pub name: String,
    pub addr: String,
}

struct ApiState {
    pub recursor: Recursor,
}

impl ApiState {
    fn new(recursor: Recursor) -> Self {
        Self { recursor }
    }
}

async fn add_forward(
    req: web::Json<AddForwardRequest>,
    zones: web::Data<ApiState>,
) -> HttpResponse {
    if let Ok(name) = Name::new(req.name.as_ref()) {
        if let Ok(addr) = req.addr.parse::<SocketAddr>() {
            zones.recursor.add_forward(name, addr);
        }
    }
    HttpResponse::Ok().json(req.0)
}

pub async fn start_recursor_api(recursor: Recursor, addr: SocketAddr) {
    HttpServer::new(move || {
        let recursor = recursor.clone();
        App::new()
            .app_data(web::JsonConfig::default().content_type(|_| true))
            .app_data(web::Data::new(ApiState::new(recursor)))
            .service(web::resource("/AddForward").route(web::post().to(add_forward)))
    })
    .bind(addr)
    .unwrap()
    .run()
    .await
    .unwrap()
}
