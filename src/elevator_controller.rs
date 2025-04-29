use core::time;
use std::{collections::{HashMap, VecDeque}, fmt::Error, sync:: Arc, thread::sleep};

use tokio::sync::broadcast::{Sender, Receiver};
use tokio::sync::Mutex;

use crate::{elevator::{Elevator, ElevatorState}, interfaces::{ElevatorControllerI, ElevatorI}};

#[derive(Debug, Clone)]
pub struct ElevatorController {
    pub elevator_id: usize,
    pub elevator: Arc<Mutex<Elevator>>,
    pub destination_map: Arc<Mutex<HashMap<usize, bool>>>,
    pub destination_list:Arc<Mutex<VecDeque<usize>>>,
    pub state_transmitter: Sender<ElevatorState>,
}

impl ElevatorController {
    pub async fn listen_request(&self, signal_receiver: Receiver<usize>) {
        let mut bind = signal_receiver;
        loop {
            match bind.recv().await{
                Ok(signal)=> {
                    println!("new request for {}, to go to {}", self.elevator_id, signal);
                    let destinations =  self.destination_map.lock().await;
                    let in_destination_list = destinations.get(&signal);
                    match in_destination_list {
                        Some(_) => {
                            continue;
                        },
                        None => {
                            println!("already in destination");
                        }
                    }

                    let _ = self.destination_list.lock().await.push_front(signal);
                },
                Err(e) => {
                    print!("error receiving signal {}", e )
                }
            }
        }
    }

    pub async fn serve(&self) {
        loop {
            sleep(time::Duration::from_millis(100));
            let next =  self.destination_list.lock().await.pop_back();
            match next {
                Some(floor) => {
                    let _ = self.go_to_floor(floor).await;
                },
                None => {

                }
            }
        }   
    }
}

impl ElevatorControllerI for ElevatorController {
    async fn go_to_floor(&self, destination: usize) -> Result<(), Error> {
        let mut elevator = self.elevator.lock().await;

        _ = elevator.close_door().await;
        let previous_direction = elevator.direction.clone();

        if destination > elevator.current_floor {
            elevator.direction = "up".to_string()
        } else {
            elevator.direction = "down".to_string()
        }

        elevator.is_moving = true;
        let current_floor = elevator.current_floor;

        for i in current_floor + 1..destination + 1 {
            elevator.current_floor = i;
            let ok = self.state_transmitter.send(
                ElevatorState {
                    id: elevator.id,
                    current_load: elevator.current_load,
                    current_floor: elevator.current_floor,
                    direction:elevator.direction.clone(),
                    initial_direction: previous_direction.clone(),
                }
            );

            tokio::task::yield_now().await;

            match ok {
                Ok(no) => {
                    // println!("{} moving to {} : now at {} published: received by {}", elevator.id, destination, elevator.current_floor, no);
                },
                Err(e) => {
                    println!("got error on publishing elevator state {} {}", elevator.id, e);
                }
            }
            sleep(time::Duration::from_secs(1));
        }

        // self.destination_map.remove(&elevator.current_floor);
        elevator.is_moving = false;
        _ = elevator.open_door().await;

        // wait for 3 seconds before closing door
        sleep(time::Duration::from_secs(3));
        _ = elevator.close_door().await;

        if self.destination_list.lock().await.len() == 0 as usize {
            let ok = self.state_transmitter.send(
                ElevatorState {
                    id: elevator.id,
                    current_load: elevator.current_load,
                    direction:"idle".to_string(),
                    current_floor: elevator.current_floor,
                    initial_direction:elevator.direction.clone(),
                }
            );

            tokio::task::yield_now().await;

            match ok {
                Ok(no) => {
                    // println!("{} moving to {} : now at {} published: received by {}", elevator.id, destination, elevator.current_floor, no);
                },
                Err(e) => {
                    println!("got error on publishing elevator state {} {}", elevator.id, e);
                }
            }
        } else {
            
        }

        Ok(())
    }
}
