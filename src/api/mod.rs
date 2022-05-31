use actix_web::{error, middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use r53::{Name, RRset};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::auth::Auth;

#[derive(Serialize, Deserialize, Debug, Default)]
struct AddZoneRequest {
    #[serde(rename = "name")]
    pub name: String,
    pub ips: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct AddRRsetRequest {
    pub zone: String,
    pub name: String,
    pub r#type: String,
    pub rdata: String,
}

struct ApiState {
    pub auth: Auth,
}

impl ApiState {
    fn new(auth: Auth) -> Self {
        Self { auth }
    }
}

async fn addzone(req: web::Json<AddZoneRequest>, zones: web::Data<ApiState>) -> HttpResponse {
    if let Ok(name) = Name::new(req.name.as_ref()) {
        zones.auth.add_zone(name, &req.ips);
    }
    HttpResponse::Ok().json(req.0)
}

pub async fn start(auth: Auth) {
    HttpServer::new(move || {
        let auth = auth.clone();
        App::new()
            .app_data(web::JsonConfig::default().content_type(|_| true))
            .app_data(web::Data::new(ApiState::new(auth)))
            .service(web::resource("/addzone").route(web::post().to(addzone)))
    })
    .bind(("127.0.0.1", 8080))
    .unwrap()
    .run()
    .await
    .unwrap()
}
