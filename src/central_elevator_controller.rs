use std::collections::VecDeque;
use std::thread::sleep;
use std::time::{self, Duration};
use std::{collections::HashMap, fmt::Error, sync::Arc};

use crate::elevator::ElevatorState;
use crate::elevator_controller::ElevatorController;
use crate::elevator_pools::elevator_queue::ElevatorQueue;
use crate::interfaces::CentralElevatorControllerI;
use crate::{elevator::Elevator, interfaces::ElevatorPool};
use tokio::sync::Mutex;
use tokio::sync::broadcast::Sender;
use tokio::sync::broadcast::{Receiver, channel};

/* Elevator controller */
/* 1. Hold all the elevator controllers */
/* 2. Storing their state based on their respective state */
#[derive(Debug)]
pub struct CentralElevatorController {
    moving_up_elevators: Mutex<ElevatorQueue>,
    moving_down_elevators: Mutex<ElevatorQueue>,
    idle_elevators: Mutex<ElevatorQueue>,
    signal_transmitter: HashMap<usize, Sender<usize>>,
    elevators: HashMap<usize, Mutex<ElevatorController>>,
}

impl CentralElevatorController {
    pub async fn listen_elevator_state(&self, state_receiver: Receiver<ElevatorState>) {
        let mut bind = state_receiver;

        loop {
            match bind.recv().await {
                Ok(state) => {
                    // let mut elevator_controller: Option<ElevatorController> = None;
                    println!("STATE: {} floor {}", state.id, state.current_floor);

                    /* adjust elevator state */
                    match state.direction.as_str() {
                        "up" => {
                            let _ = self.moving_up_elevators.lock().await.insert_elevator(state.clone()).await;
                        },
                        "down" => {
                            let _ = self.moving_down_elevators.lock().await.insert_elevator(state.clone()).await;

                        },
                        "idle" => {
                            let _ = self.idle_elevators.lock().await.insert_elevator(state.clone()).await;
                        },
                        &_ =>{}
                    }

                    /* an elevator becomes idle */
                    match state.direction.as_str() == "idle" {
                        true => {
                            match state.initial_direction.as_str() {
                                "up" => {
                                    self.moving_up_elevators.lock().await.remove_elevator(state.id).await;
                                },
                                "down"=> {
                                    self.moving_down_elevators.lock().await.remove_elevator(state.id).await;
                                },
                                &_ => {}
                            }
                        },
                        false => {

                        }
                    }
                }
                Err(e) => {
                    println!("Failed to get elevator state");
                }
            }

            sleep(time::Duration::from_secs(1));
        }
    }

    pub async fn new() -> Arc<CentralElevatorController> {
        /* Elevator containers */
        let mut idle_elevators = ElevatorQueue::new();
        let mut elevator_controllers: HashMap<usize, Mutex<ElevatorController>> = HashMap::new();

        let mut signal_transmitter: HashMap<usize, Sender<usize>> = HashMap::new();
        let mut state_receivers: Vec<Receiver<ElevatorState>> = Vec::new();

        /* Building elevators */
        for i in 0..2 {
            let (state_tx, state_rx): (Sender<ElevatorState>, Receiver<ElevatorState>) = channel(1);
            let shared = Arc::new(Mutex::new(Elevator::new(i)));

            state_receivers.insert(i, state_rx);

            let (signal_tx, signal_rx): (Sender<usize>, Receiver<usize>) = channel(1);
            signal_transmitter.insert(i, signal_tx);

            /* Put the elevator to idles elevator */
            let elevator_controller = ElevatorController {
                elevator: shared,
                elevator_id: i,
                destination_map: HashMap::new(),
                destination_list: Arc::new(Mutex::new(VecDeque::new())),
                state_transmitter: state_tx,
            };

            /* Runner for receiving requests from central controller */
            let elevator_request_listener_bind = elevator_controller.clone();
            tokio::spawn(async move {
                elevator_request_listener_bind.listen_request(signal_rx).await;
            });

            let elevator_request_serve_bind = elevator_controller.clone();
            tokio::spawn(async move {
                elevator_request_serve_bind.serve().await;
            });

            elevator_controllers.insert(i, Mutex::new(elevator_controller));

            let _ = idle_elevators
                .insert_elevator(ElevatorState {
                    id: i,
                    current_floor: 0,
                    current_load: 0,
                    direction: "idle".to_string(),
                    initial_direction: "idle".to_string(),
                })
                .await;
            // idle_elevators.elevators_index.lock().await.insert(i, i);
        }

        let controller = Arc::new(CentralElevatorController {
            moving_down_elevators: Mutex::new(ElevatorQueue::new()),
            moving_up_elevators: Mutex::new(ElevatorQueue::new()),
            idle_elevators: Mutex::new(idle_elevators),
            signal_transmitter: signal_transmitter,
            elevators: elevator_controllers,
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
    async fn call_for_an_elevator(&self, floor: usize, direction: String) -> Result<(), Error> {
        let mut elevator: Option<ElevatorState> = None;

        /* Check if any idle elevator available */
        let idle_elevators = self.idle_elevators.lock().await;
        let idles = idle_elevators.len().await;
        drop(idle_elevators);

        if idles > 0 {
            /* Choose idle elevator */
            let mut idle_elevators = self.idle_elevators.lock().await;
            elevator = idle_elevators.get_elevator().await;
            drop(idle_elevators);
        } else if direction == "up".to_string() {
            /* Choose elevator that is moving up */
            let mut moving_up_elevators = self.moving_up_elevators.lock().await;
            elevator = moving_up_elevators.get_elevator().await;
            drop(moving_up_elevators);
        } else {
            /* Choose elevator that is moving down */
            let mut moving_down_elevators = self.moving_down_elevators.lock().await;
            elevator = moving_down_elevators.get_elevator().await;
            drop(moving_down_elevators);
        }

        match elevator {
            Some(e) => {
                /* send request to elevator */
                let signal_transmitter = self.signal_transmitter.get(&e.id);
                match signal_transmitter {
                    Some(tx) => {
                        let _ = tx.send(floor);
                    }
                    None => {}
                }
            }
            None => {
                println!("ran out of elevators!");
            }
        }

        Ok(())
    }
}
