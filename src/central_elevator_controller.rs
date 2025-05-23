use std::thread::sleep;
use std::time::{self};
use std::{collections::HashMap, fmt::Error, sync::Arc};

use crate::elevator::ElevatorState;
use crate::elevator_controller::ElevatorController;
use crate::elevator_pools::elevator_queue::ElevatorQueue;
use crate::interfaces::CentralElevatorControllerI;
use crate::interfaces::ElevatorPool;
use tokio::sync::{Mutex, Semaphore};
use tokio::sync::broadcast::Sender;
use tokio::sync::broadcast::{Receiver, channel};

#[derive(Clone)]
pub struct ElevatorRequest {
    pub from: usize,
    pub to: usize,
}

/* Elevator controller */
/* 1. Hold all the elevator controllers */
/* 2. Stores elevators based on their respective state */
#[derive(Debug)]
pub struct CentralElevatorController {
    moving_up_elevators: Mutex<ElevatorQueue>,
    moving_down_elevators: Mutex<ElevatorQueue>,
    idle_elevators: Mutex<ElevatorQueue>,
    permits: Mutex<Semaphore>,
    signal_transmitter: HashMap<usize, Sender<ElevatorRequest>>,
    global_state_tx: Sender<ElevatorState>
}

impl CentralElevatorController {
    pub async fn listen_elevator_state(&self, state_receiver: Receiver<ElevatorState>) {
        let mut bind = state_receiver;

        loop {
            match bind.recv().await {
                Ok(state) => {
                    // let mut elevator_controller: Option<ElevatorController> = None;
                    println!("STATE: {} floor {} from {} to {}", state.id, state.current_floor, state.initial_direction, state.direction);

                    let _ = self.global_state_tx.send(state.clone());

                    if state.direction.as_str() == state.initial_direction.as_str() && state.direction.as_str() != "idle" {
                        continue;
                    }

                    /* adjust elevator state */
                    match state.direction.as_str() {
                        "up" => {
                            let _ = self.moving_up_elevators.lock().await.insert_elevator(state.clone()).await;
                        },
                        "down" => {
                            let _ = self.moving_down_elevators.lock().await.insert_elevator(state.clone()).await;
                        },
                        "idle" => {
                            println!("inserted to idle {}", state.id);
                            let _ = self.idle_elevators.lock().await.insert_elevator(state.clone()).await;
                        },
                        &_ =>{}
                    }

                    match state.initial_direction.as_str() {
                        "up" => {
                            let _ = self.moving_up_elevators.lock().await.remove_elevator(state.id).await;
                        },
                        "down" => {
                            let _ = self.moving_down_elevators.lock().await.remove_elevator(state.id).await;
                        },
                        "idle" => {
                            println!("inserted to idle {}", state.id);
                            let _ = self.idle_elevators.lock().await.remove_elevator(state.id).await;
                        },
                        &_ =>{}
                    }

                }
                Err(_) => {
                    println!("Failed to get elevator state");
                }
            }

            sleep(time::Duration::from_secs(1));
        }
    }

    pub async fn new(global_state_tx : Sender<ElevatorState>, no_of_elevator: usize) -> Arc<CentralElevatorController> {
        /* Elevator containers */
        let mut idle_elevators = ElevatorQueue::new();

        let mut signal_transmitter: HashMap<usize, Sender<ElevatorRequest>> = HashMap::new();
        let mut state_receivers: Vec<Receiver<ElevatorState>> = Vec::new();

        let mut permits_size:usize = 0;

        /* Building elevators */
        for i in 0..no_of_elevator {
            let (state_tx, state_rx): (Sender<ElevatorState>, Receiver<ElevatorState>) = channel(1);
            state_receivers.insert(i, state_rx);

            let (signal_tx, signal_rx): (Sender<ElevatorRequest>, Receiver<ElevatorRequest>) = channel(10);
            signal_transmitter.insert(i, signal_tx);

            /* Runner for receiving requests from central controller */
            tokio::spawn(async move {
                let elevator_controller = ElevatorController::new(i, state_tx);
                elevator_controller.listen_request(signal_rx).await;
            });

            /* Put the elevator to idles elevator */
            let _ = idle_elevators
                .insert_elevator(ElevatorState {
                    id: i,
                    current_floor: 0,
                    current_load: 0,
                    direction: "idle".to_string(),
                    initial_direction: "idle".to_string(),
                    is_door_open: false,
                    is_moving: false,
                })
                .await;

            permits_size +=1;
        }


        let controller = Arc::new(CentralElevatorController {
            moving_down_elevators: Mutex::new(ElevatorQueue::new()),
            moving_up_elevators: Mutex::new(ElevatorQueue::new()),
            idle_elevators: Mutex::new(idle_elevators),
            signal_transmitter: signal_transmitter,
            permits: Mutex::new(Semaphore::new(permits_size)),
            global_state_tx: global_state_tx
        });

        for state_rx in state_receivers {
            let bind_controller = controller.clone();
            let bind_rx = state_rx;
            tokio::spawn(async move {
                bind_controller.listen_elevator_state(bind_rx).await;
            });
        }

        return controller;
    }
}

impl CentralElevatorControllerI for CentralElevatorController {
    async fn print_states(&self) {
        println!("idle: {:?}", self.idle_elevators.lock().await.elevators.lock().await);
        println!("up: {:?}", self.moving_up_elevators.lock().await.elevators.lock().await);
        println!("down: {:?}", self.moving_down_elevators.lock().await.elevators.lock().await);
    }

    async fn call_for_an_elevator(&self, floor: usize, destination: usize) -> Result<usize, Error> {
        let _ = self.permits.lock().await.acquire().await;
        let mut elevator: Option<ElevatorState> = None;

        let mut direction = "up".to_string();
        if floor > destination {
            direction = "down".to_string();
        }

        /* Check if any idle elevator available */
        let mut idle_elevators = self.idle_elevators.lock().await;
        let idles = idle_elevators.len().await;

        if idles > 0 {
            /* Choose idle elevator */
            elevator = idle_elevators.get_elevator().await;
            drop(idle_elevators);
        } else if direction == "up".to_string() {
            /* Choose elevator that is moving up */
            let mut moving_up_elevators = self.moving_up_elevators.lock().await;
            elevator = moving_up_elevators.get_elevator().await;
        } else {
            /* Choose elevator that is moving down */
            let mut moving_down_elevators = self.moving_down_elevators.lock().await;
            elevator = moving_down_elevators.get_elevator().await;
        }

        match elevator.clone() {
            Some(e) => {
                /* send request to elevator */
                let signal_transmitter = self.signal_transmitter.get(&e.id);
                match signal_transmitter {
                    Some(tx) => {
                        let _ = tx.send(ElevatorRequest{
                            from: floor,
                            to: destination,
                        });
                        return Ok(e.id);
                    }
                    None => {
                        return Ok(123 as usize);
                    }
                }
            }
            None => {
                println!("ran out of elevators!");
            }
        }

        Ok(123 as usize)
    }
}
