use std::net::SocketAddr;

use actix_web::{error, middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use r53::{Name, RRset};
use serde::{Deserialize, Serialize};

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
    pub rrset: Vec<String>,
}

struct ApiState {
    pub auth: Auth,
}

impl ApiState {
    fn new(auth: Auth) -> Self {
        Self { auth }
    }
}

async fn add_zone(req: web::Json<AddZoneRequest>, zones: web::Data<ApiState>) -> HttpResponse {
    if let Ok(name) = Name::new(req.name.as_ref()) {
        zones.auth.add_zone(name, &req.ips);
    }
    HttpResponse::Ok().json(req.0)
}

async fn add_rrset(req: web::Json<AddRRsetRequest>, zones: web::Data<ApiState>) -> HttpResponse {
    if let Ok(name) = Name::new(req.zone.as_ref()) {
        if let Ok(rrset) = RRset::from_strs(&req.rrset) {
            zones.auth.add_rrset(&name, rrset);
        }
    }
    HttpResponse::Ok().json(req.0)
}

pub async fn start_auth_api(auth: Auth, addr: SocketAddr) {
    HttpServer::new(move || {
        let auth = auth.clone();
        App::new()
            .app_data(web::JsonConfig::default().content_type(|_| true))
            .app_data(web::Data::new(ApiState::new(auth)))
            .service(web::resource("/addzone").route(web::post().to(add_zone)))
            .service(web::resource("/addrrset").route(web::post().to(add_rrset)))
    })
    .bind(addr)
    .unwrap()
    .run()
    .await
    .unwrap()
}
