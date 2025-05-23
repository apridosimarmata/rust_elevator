use std::{collections::HashMap, convert::Infallible, sync::Arc, time::Duration, usize};

use actix_web::{web::{self, ServiceConfig}, HttpRequest, HttpResponse, Responder};
use actix_web_lab::sse::{self, Event};
use futures::lock::Mutex;
use tokio::sync::{broadcast::Receiver as BroadcastReceiver, mpsc::{Sender, Receiver, channel}};
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::ReceiverStream;

use crate::{central_elevator_controller::CentralElevatorController, elevator::ElevatorState, interfaces::CentralElevatorControllerI};


struct Visitor {
    elevator: Option<usize>,
    floor: Option<usize>,

    id: String, 
}

pub struct ElevatorHTTPHandlerImpl {
    central_elevator_controller: Arc<CentralElevatorController>,
    global_state_rx: Arc<BroadcastReceiver<ElevatorState>>,
    visitors: Mutex<HashMap<String, Mutex<Visitor>>>,
    elevator_passenger: Mutex<HashMap<usize, Mutex<HashMap<String, bool>>>>
}

pub trait ElevatorHTTPHandler {
    async fn get_elevator(&self, from : usize, destination : usize) -> usize;
    async fn listen_state(&self, tx : &Sender< Result<Event, Infallible>>);
    async fn print_elevator_state(&self) -> impl Responder;
}

pub fn register_job_routes(router_config: &mut ServiceConfig, elevator_controller:  Arc<CentralElevatorController>, global_state_rx: Arc<BroadcastReceiver<ElevatorState>>) {
    let job_http_handler = ElevatorHTTPHandlerImpl {
        central_elevator_controller: elevator_controller,
        global_state_rx:global_state_rx,
        visitors: Mutex::new(HashMap::new()),
        elevator_passenger: Mutex::new(HashMap::new())
    };

    router_config.app_data(web::Data::new(job_http_handler))
       .service(
            web::scope("/api/v1") 
            .route("/elevator/stream", web::get().to(|data: web::Data<ElevatorHTTPHandlerImpl>| async move {
                let (tx, rx) : (Sender<Result<Event, Infallible>>, Receiver<Result<Event, Infallible>>) = channel(10);

                data.listen_state(&tx).await;
                let data_stream: ReceiverStream<Result<Event, Infallible>> = ReceiverStream::new(rx);
                return sse::Sse::from_stream(data_stream).with_keep_alive(Duration::from_secs(5))
            }))
            .route("/elevator/state", web::get().to(|data: web::Data<ElevatorHTTPHandlerImpl>| async move {
                let _ = data.print_elevator_state().await;

                HTTPResponder::Ok(())
            }))
            .route("/elevator/{destination}", web::get().to(|data: web::Data<ElevatorHTTPHandlerImpl>, path: web::Path<i32>, req: HttpRequest| async move {
                    let visitor_id = if let Some(cookie) = req.cookie("visitor_id") {
                        cookie.value().to_string()
                    } else {
                        return HTTPResponder::Ok(())
                    };

                    let mut elevator_id: usize = 123;

                    let mut visitors = data.visitors.lock().await;
                    let destination = path.into_inner() as usize;
                    match visitors.get(&visitor_id.clone()) {
                        Some(v) => {
                            match v.lock().await.floor {
                                Some(floor) => {
                                    elevator_id = data.get_elevator(floor, destination).await;
                                },
                                None => {

                                }
                            }
                        },
                        None => {
                            let visitor = Visitor { elevator: None, floor: Some(0), id: visitor_id.clone() };
                            visitors.insert(visitor_id.clone(), Mutex::new(visitor));
                            elevator_id = data.get_elevator(0, destination).await;
                        }
                    }


                    HTTPResponder::OkWithElevatorId(elevator_id)
                })),
        );
}

impl ElevatorHTTPHandlerImpl {

}

impl ElevatorHTTPHandler for ElevatorHTTPHandlerImpl {
    async fn get_elevator (&self,  from : usize, destination : usize) ->  usize{
        let result = self.central_elevator_controller.call_for_an_elevator(from, destination).await;

        match result {
            Ok(id) => {
                return id
            },
            Err(_) =>{

            }
        }

        return 123
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
    OkWithElevatorId(usize),
    BadRequest(String),
    InternalServerError(String)
}

impl<T: Serialize> Responder for HTTPResponder<T> {
    type Body = actix_web::body::BoxBody;

    fn respond_to(self, _req: &actix_web::HttpRequest) -> HttpResponse<Self::Body> {
        match self {
            HTTPResponder::Ok(data) => HttpResponse::Ok().json(CustomHTTPResponse { data }),
            HTTPResponder::BadRequest(msg) => HttpResponse::BadRequest().json(CustomHTTPError { error: msg }),
            HTTPResponder::InternalServerError(msg) => HttpResponse::InternalServerError().json(CustomHTTPError {error: msg}),
            HTTPResponder::OkWithElevatorId(data) =>  HttpResponse::Ok().json(CustomHTTPResponse { data }),
        }
    }
}