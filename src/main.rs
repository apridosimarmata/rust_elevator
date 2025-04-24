use std::{ sync::Arc, time};
use central_elevator_controller::CentralElevatorController;
use interfaces::{CentralElevatorControllerI, ElevatorControllerI};
use tokio::time::sleep;


mod elevator_heap;
mod interfaces;
mod elevator;
mod central_elevator_controller;
mod elevator_controller;

#[tokio::main]
async fn main() {

    /* elevators_state_stream */
    // let (tx, rx) : (Sender<Vec<Elevator>>, Receiver<Vec<Elevator>>) = channel(10);

    let elevator_controller = Arc::new(CentralElevatorController::new().await);



    let one =  elevator_controller.clone();
    let two =  elevator_controller.clone();
    let three =  elevator_controller.clone();
    let four =  elevator_controller.clone();
    let five =  elevator_controller.clone();


    tokio::spawn(async move {
        let _ = one.call_for_an_elevator(3, "up".to_string()).await;
    });

    tokio::spawn(async move  {
        let _ = two.call_for_an_elevator(4, "up".to_string()).await;
    });

    tokio::spawn(async move  {
        let _ = three.call_for_an_elevator(2, "down".to_string()).await;
    });

    sleep(time::Duration::from_secs(1)).await;

    tokio::spawn(async move  {
        let _ = four.call_for_an_elevator(2, "down".to_string()).await;
    });

    tokio::spawn(async move  {
        let _ = five.call_for_an_elevator(1, "down".to_string()).await;
    });

    loop {
        
    }
}