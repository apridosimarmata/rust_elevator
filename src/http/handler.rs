use std::{convert::Infallible, sync::Arc, time::Duration};

use actix_web::{web::{self, ServiceConfig}, HttpResponse, Responder};
use actix_web_lab::sse::{self, Event};
use tokio::sync::{broadcast::Receiver as BroadcastReceiver, mpsc::{Sender, Receiver, channel}};
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::ReceiverStream;

use crate::{central_elevator_controller::CentralElevatorController, elevator::ElevatorState, interfaces::CentralElevatorControllerI};

pub struct ElevatorHTTPHandLerImpl {
    central_elevator_controller: Arc<CentralElevatorController>,
    global_state_rx: Arc<BroadcastReceiver<ElevatorState>>
}

pub trait ElevatorHTTPHandler {
    async fn get_elevator(&self, requested_floor : usize) ->impl Responder;
    async fn listen_state(&self, tx : &Sender< Result<Event, Infallible>>);
    async fn print_elevator_state(&self) -> impl Responder;
}

pub fn register_job_routes(router_config: &mut ServiceConfig, elevator_controller:  Arc<CentralElevatorController>, global_state_rx: Arc<BroadcastReceiver<ElevatorState>>) {
    let job_http_handler = ElevatorHTTPHandLerImpl {
        central_elevator_controller: elevator_controller,
        global_state_rx:global_state_rx,
    };

    router_config.app_data(web::Data::new(job_http_handler))
       .service(
            web::scope("/api/v1") 
            .route("/elevator/stream", web::get().to(|data: web::Data<ElevatorHTTPHandLerImpl>| async move {
                let (tx, rx) : (Sender<Result<Event, Infallible>>, Receiver<Result<Event, Infallible>>) = channel(10);

                data.listen_state(&tx).await;
                let data_stream: ReceiverStream<Result<Event, Infallible>> = ReceiverStream::new(rx);
                return sse::Sse::from_stream(data_stream).with_keep_alive(Duration::from_secs(5))
            }))
            .route("/elevator/state", web::get().to(|data: web::Data<ElevatorHTTPHandLerImpl>| async move {
                let y = data.print_elevator_state().await;

                HTTPResponder::Ok(())
            }))
            .route("/elevator/{elevator_id}", web::get().to(|data: web::Data<ElevatorHTTPHandLerImpl>, path: web::Path<i32>| async move {
                    let requested_floor = path.into_inner() as usize;
                    let y = data.get_elevator(requested_floor).await;

                    HTTPResponder::Ok(())
                })),
        );
}

impl ElevatorHTTPHandLerImpl {

}

impl ElevatorHTTPHandler for ElevatorHTTPHandLerImpl {
    async fn get_elevator (&self, requested_floor : usize) ->  impl Responder{
        let bind = self.central_elevator_controller.clone();

        tokio::spawn(async move {
            let _ = bind.call_for_an_elevator(requested_floor, "up".to_string()).await;
        });

        HTTPResponder::Ok(())
    }
    
    async fn listen_state(&self, tx : &Sender< Result<Event, Infallible>>) {
        /* listen from global rx */

        let mut rx: BroadcastReceiver<ElevatorState> = self.global_state_rx.resubscribe();
        let tx_cloned = tx.clone();

        tokio::spawn(async move{
            loop {
                match rx.recv().await {
                    Ok(state) => {
                        let json_val = serde_json::to_string(&state).map_or_else(|e| {
                            return Err(e.to_string())
                        }, Ok);
    
                        let data = Event::Data(
                            sse::Data::new(json_val.unwrap()),
                        );
                        let event = Ok::<_, Infallible>(data);
                        match tx_cloned.send(event).await {
                            Ok(_) => {},
                            Err(_) =>{}
                        } ;
                    },
                    Err(e) => {
    
                    }
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
        
    }
    
    async fn print_elevator_state(&self) -> impl Responder {
        let bind = self.central_elevator_controller.clone();

        tokio::spawn(async move {
            let _ = bind.print_states().await;
        });

        HTTPResponder::Ok(())
    }
}


#[derive(Serialize, Deserialize)]
pub struct CustomHTTPResponse<T: Serialize> {
    pub data: T,
}

#[derive(Serialize, Deserialize)]
pub struct CustomHTTPError {
    pub error: String,
}

#[derive(Serialize, Deserialize)]
pub enum HTTPResponder<T: Serialize> {
    Ok(T),
    BadRequest(String),
    InternalServerError(String)
}

impl<T: Serialize> Responder for HTTPResponder<T> {
    type Body = actix_web::body::BoxBody;

    fn respond_to(self, _req: &actix_web::HttpRequest) -> HttpResponse<Self::Body> {
        match self {
            HTTPResponder::Ok(data) => HttpResponse::Ok().json(CustomHTTPResponse { data }),
            HTTPResponder::BadRequest(msg) => HttpResponse::BadRequest().json(CustomHTTPError { error: msg }),
            HTTPResponder::InternalServerError(msg) => HttpResponse::InternalServerError().json(CustomHTTPError {error: msg})
        }
    }
}