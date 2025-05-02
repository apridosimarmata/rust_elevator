use std::{path::PathBuf, sync::Arc};

use actix_files::NamedFile;
use actix_web::{cookie::{Cookie, SameSite}, dev::{Service, ServiceRequest, ServiceResponse, Transform}, http::Error, web, App, HttpRequest, HttpResponse, HttpServer, Result};
use central_elevator_controller::CentralElevatorController;
use elevator::ElevatorState;
use futures::future::Ready;
use http::handler::register_job_routes;
use tokio::sync::broadcast::{channel, Receiver, Sender};
use uuid::Uuid;

mod elevator_pools;
mod interfaces;
mod elevator;
mod central_elevator_controller;
mod elevator_controller;
mod http;

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    /* elevators_state_stream */
    let (tx, rx) : (Sender<ElevatorState>, Receiver<ElevatorState>) = channel(10);
    let elevator_controller = CentralElevatorController::new(tx.clone(), 3).await;

    let rx_bind = Arc::new(rx);
    HttpServer::new(move || {
        App::new()
        .configure(|cfg| register_job_routes(cfg, elevator_controller.clone(), rx_bind.clone()))
        .route("/", web::get().to(index))
    })
    .bind("0.0.0.0:3000")?
    .run()
    .await

}

async fn index(_req: HttpRequest) -> Result<HttpResponse> {
    let visitor_id = _req.cookie("visitor_id");

    println!("{:?}", visitor_id);

    let path: PathBuf = "./static/index.html".parse().unwrap();
    let named_file = NamedFile::open(path)?;

    match visitor_id {
        Some(_) => {
            Ok(named_file.into_response(&_req))
        },
        None => {
            let new_visitor_id = Uuid::new_v4().to_string();
            let mut cookie = Cookie::new("visitor_id", new_visitor_id);
            cookie.set_path("/");
            cookie.set_same_site(SameSite::Lax);

            let mut response = named_file.into_response(&_req);
            response.add_cookie(&cookie).unwrap();

            Ok(response)
        }
    }

}