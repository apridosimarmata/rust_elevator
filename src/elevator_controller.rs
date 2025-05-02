use std::{
    collections::{HashMap, VecDeque},
    fmt::Error,
    sync::Arc,
};

use tokio::time::{sleep, Duration};

use tokio::sync::Mutex;
use tokio::sync::broadcast::{Receiver, Sender};

use crate::{
    elevator::ElevatorState,
    interfaces::{ElevatorControllerI, ElevatorI},
};

#[derive(Debug, Clone)]
pub struct ElevatorController {
    pub state: Arc<Mutex<ElevatorState>>,

    pub destination_map: Arc<Mutex<HashMap<usize, bool>>>,
    pub destination_list: Arc<Mutex<VecDeque<usize>>>,
    
    pub state_transmitter: Sender<ElevatorState>, /* used to send state to central controller */

    is_busy: Arc<Mutex<bool>>,
}

impl ElevatorController {
    pub fn new(id: usize, state_tx: Sender<ElevatorState>) -> Self {
        return ElevatorController {
            state: Arc::new(Mutex::new(ElevatorState::new(id))),
            destination_map: Arc::new(Mutex::new(HashMap::new())),
            destination_list: Arc::new(Mutex::new(VecDeque::new())),
            state_transmitter: state_tx,
            is_busy: Arc::new(Mutex::new(false)),
        };
    }

    /* Receive a channel receiver and listen to each request made by central controller */
    pub async fn listen_request(&self, signal_receiver: Receiver<usize>) {
        let mut bind = signal_receiver;

        loop {
            match bind.recv().await {
                Ok(requested_floor) => {
                    /* drop request as already in request queue */
                    let mut destination_map = self.destination_map.lock().await;
                    let in_destination_list = destination_map.get(&requested_floor);
                    match in_destination_list {
                        Some(_) => {
                            continue;
                        }
                        None => {}
                    }

                    /* append the request to the queue */
                    let mut destination_list = self.destination_list.lock().await;
                    destination_list.push_front(requested_floor);
                    destination_map.insert(requested_floor, true);

                    /* if busy, quit. will be processed soon */
                    let mut busy = self.is_busy.lock().await;
                    if *busy {
                        continue;
                    }

                    *busy = true;
                    drop(busy);
                    drop(destination_list);
                    drop(destination_map);

                    /* if not busy, process this request */
                    /* and while there's a queued request (probably by another thread), keep going */
                    loop {
                        let mut destination_list = self.destination_list.lock().await;
                        if destination_list.len() == 0 {
                            break;
                        }

                        let next = destination_list.pop_back();
                        drop(destination_list);

                        match next {
                            Some(n) => {
                                let _ = self.go_to_floor(n).await;

                                /* remove from destination map */
                                let mut destination_map = self.destination_map.lock().await;
                                destination_map.remove(&n);
                            }
                            None => {}
                        }
                    }

                    let mut busy = self.is_busy.lock().await;
                    *busy = false
                }
                Err(e) => {
                    print!("error receiving signal {}", e)
                }
            }
        }
    }
}

impl ElevatorControllerI for ElevatorController {
    async fn go_to_floor(&self, destination: usize) -> Result<(), Error> {
        let mut elevator = self.state.lock().await;

        if destination == elevator.current_floor {
            let _ = elevator.open_door();
            return Ok(())
        }

        elevator.initial_direction = elevator.direction.clone();

        if destination > elevator.current_floor {
            elevator.direction = "up".to_string()
        } else {
            elevator.direction = "down".to_string()
        }

        elevator.is_moving = true;

        let mut current_floor =  elevator.current_floor;
        loop {
            /* artificial delay, mimick a moving elevator */
            sleep(Duration::from_millis(1500)).await;
            elevator.current_floor = current_floor;

            /* send the state after movement */
            let ok = self.state_transmitter.send(elevator.clone());
            match ok {
                Ok(_) => {

                }
                Err(e) => {
                    println!(
                        "got error on publishing elevator state {} {}",
                        elevator.id, e
                    );
                }
            }
            tokio::task::yield_now().await;

            elevator.initial_direction = elevator.direction.clone();
            if current_floor == destination {
                println!("Elevator {} arrived at destination {}", elevator.id, destination);
                break
            }

            /* decide where to go next */
            if elevator.direction == "up".to_string() {
                current_floor = current_floor + 1;
            }else if elevator.direction == "down".to_string(){
                current_floor = current_floor - 1;
            }
        }

        tokio::task::yield_now().await;
        elevator.is_moving = false;

        /* open and close the door */
        _ = elevator.open_door().await;

        let _ = self.state_transmitter.send(elevator.clone());
        tokio::task::yield_now().await;

        sleep(Duration::from_secs(5)).await;
        _ = elevator.close_door().await;

        /* elevator becomes idle? */
        if self.destination_list.lock().await.len() == 0 as usize {
            println!("Elevator becomes idle: {}", elevator.id);
            elevator.direction = "idle".to_string();

            /* send the state after idle */
            let ok = self.state_transmitter.send(elevator.clone());
            tokio::task::yield_now().await;

            match ok {
                Ok(_) => {

                }
                Err(e) => {
                    println!(
                        "got error on publishing elevator state {} {}",
                        elevator.id, e
                    );
                }
            }
        }

        Ok(())
    }
}
