use std::{path::PathBuf, sync::Arc};

use actix_files::NamedFile;
use actix_web::{web, App, HttpRequest, HttpServer, Result};
use central_elevator_controller::CentralElevatorController;
use elevator::{ElevatorState};
use http::handler::register_job_routes;
use tokio::sync::broadcast::{channel, Receiver, Sender};


mod elevator_pools;
mod interfaces;
mod elevator;
mod central_elevator_controller;
mod elevator_controller;
mod http;


pub struct SharedData {
    global_state_rx: Receiver<ElevatorState>,
    central_controller: Arc<CentralElevatorController>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    /* elevators_state_stream */
    let (tx, rx) : (Sender<ElevatorState>, Receiver<ElevatorState>) = channel(10);
    let elevator_controller = CentralElevatorController::new(tx.clone()).await;

    let rx_bind = Arc::new(rx);


    HttpServer::new(move || {
        App::new()
        // .app_data(shared_data.clone())
        .configure(|cfg| register_job_routes(cfg, elevator_controller.clone(),rx_bind.clone()))
        .route("/", web::get().to(index))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await

}


async fn index(_req: HttpRequest) -> Result<NamedFile> {
    let path: PathBuf = "./static/index.html".parse().unwrap();
    Ok(NamedFile::open(path)?)
}