use std::collections::VecDeque;
use std::os::macos::raw::stat;
use std::{collections::HashMap, fmt::Error, sync::Arc};

use tokio::sync::broadcast::Sender;
use tokio::sync::Mutex;
use tokio::sync::broadcast::{Receiver, channel};
use crate::elevator::ElevatorState;
use crate::elevator_controller::ElevatorController;
use crate::interfaces::CentralElevatorControllerI;
use crate::{
    elevator::Elevator,
    elevator_heap::ElevatorHeap,
    interfaces::ElevatorHeapI,
};

#[derive(Debug)]
pub struct CentralElevatorController {
    moving_up_elevators: Mutex<ElevatorHeap>,
    moving_down_elevators: Mutex<ElevatorHeap>,
    idle_elevators: Mutex<ElevatorHeap>,
    signal_transmitter: HashMap<usize, Sender<usize>>,
}

impl CentralElevatorController {
    pub async fn listen_elevator_state(&self, state_receiver: Receiver<ElevatorState>){
        let mut bind = state_receiver;

        loop{
            match bind.recv().await{
                Ok(state) => {
                    // println!("elevator {}: is at {} floor with current load {} and moving {} {}", state.id, state.current_floor, state.current_load, state.previous_direction, state.previous_direction.as_str() == "idle");

                    println!("direction : {}\nprev direction: {} ", state.direction, state.previous_direction);

                    let mut elevator_controller: Option<ElevatorController> = None;

                    match state.previous_direction.as_str() {
                        "idle" => {
                            println!("removed from idle");
                            elevator_controller = self.idle_elevators.lock().await.remove_elevator(state.id).await;
                        },
                        "up" => {
                            elevator_controller = self.moving_up_elevators.lock().await.remove_elevator(state.id).await;
                        },
                        "down" => {
                            elevator_controller = self.moving_down_elevators.lock().await.remove_elevator(state.id).await;
                        },
                        &_ => {}
                    }

                    println!("her! {} {:?}", state.direction.as_str(), elevator_controller);
                    match elevator_controller.is_some() {
                        true => {
                            match state.direction.as_str() {
                                "idle" => {
                                    println!("should be inserted to idle");
                                    let _ = self.idle_elevators.lock().await.insert_elevator(elevator_controller.unwrap()).await;
                                },
                                "up" => {
                                    println!("should be inserted to up");
                                    let _ = self.moving_up_elevators.lock().await.insert_elevator(elevator_controller.unwrap()).await;
                                },
                                "down" => {
                                    let _ = self.moving_down_elevators.lock().await.insert_elevator(elevator_controller.unwrap()).await;
                                },
                                &_ => {}
                            }
                        },
                        false => {
                            println!("her!2");
                        }
                    }
                },
                Err(e) => {
                    println!("got error when receiving elevator state {}", e);
                }
            }

            // println!("idle elevators {:?}", self.idle_elevators);
            // println!("ups {:?}", self.moving_up_elevators);
            // println!("downs {:?}", self.moving_down_elevators);

            println!("");

        }
    }

    pub async fn new(
    ) -> Arc<CentralElevatorController> {
        let mut idle_elevators = ElevatorHeap::new();
        let mut signal_transmitter: HashMap<usize, Sender<usize>>=  HashMap::new();
        let mut state_receivers: Vec<Receiver<ElevatorState>> = Vec::new();

        /* Building elevators */
        for i in 0..2 {
            let (state_tx, state_rx): (Sender<ElevatorState>, Receiver<ElevatorState>) = channel(1);
            let shared = Arc::new(Mutex::new(Elevator::new(i)));

            state_receivers.insert(i, state_rx);

            let (signal_tx, signal_rx): (Sender<usize>, Receiver<usize>) = channel(1);
            signal_transmitter.insert(i, signal_tx);

            /* Put the elevator to idles elevator */
            let elevator_controller = ElevatorController{
                elevator: shared,
                elevator_id: i,
                destination_map: HashMap::new(),
                destination_list: VecDeque::new(),
                state_transmitter: state_tx,
            };

            /* Runner for receiving requests from central controller */
            let runner_controller_bind = elevator_controller.clone();
            tokio::spawn(async move {
                runner_controller_bind.run(signal_rx).await;
            });

            let _ = idle_elevators.insert_elevator(elevator_controller).await;
            idle_elevators.elevators_index.lock().await.insert(i, i);
        }


        let controller = Arc::new(
            CentralElevatorController {
                moving_down_elevators: Mutex::new(ElevatorHeap::new()),
                moving_up_elevators: Mutex::new(ElevatorHeap::new()),
                idle_elevators: Mutex::new(idle_elevators),
                signal_transmitter: signal_transmitter,
            }
        );

        for state_rx in state_receivers {
            let bind_controller = controller.clone();
            let bind_rx = state_rx;
            tokio::spawn(async move {
                bind_controller.listen_elevator_state(bind_rx).await;
            });
        }

        return controller
    }

}

impl CentralElevatorControllerI for CentralElevatorController {

    async fn call_for_an_elevator(&self, floor: usize, direction: String) -> Result<(), Error> {
        let elevator: Option<ElevatorController>;

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
            // send request to selected elevator
            Some(e) => {
                let elevator = e.elevator.lock().await;
                let elevator_id = elevator.id;
                drop(elevator);

                let transmitter = self.signal_transmitter.get(&elevator_id);
                match transmitter {
                    Some(t) => {
                        let _ = t.send(floor);
                    },
                    None => {

                    },
                }

                return Ok(())
            }
            None => {}
        }


        Ok(())
    }
}
