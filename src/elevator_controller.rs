use core::time;
use std::{collections::{HashMap, VecDeque}, fmt::Error, sync:: Arc, thread::sleep};

use tokio::sync::broadcast::{Sender, Receiver};
use tokio::sync::Mutex;

use crate::{elevator::{Elevator, ElevatorState}, interfaces::{ElevatorControllerI, ElevatorI}};

#[derive(Debug, Clone)]
pub struct ElevatorController {
    pub elevator_id: usize,
    pub elevator: Arc<Mutex<Elevator>>,
    pub destination_map: HashMap<usize, bool>,
    pub destination_list: VecDeque<usize>,
    pub state_transmitter: Sender<ElevatorState>,
}

impl ElevatorController {
    pub async fn run(&self, signal_receiver: Receiver<usize>) {
        let mut bind = signal_receiver;
        loop {
            match bind.recv().await{
                Ok(signal)=> {
                    sleep(time::Duration::from_secs(1));
                    let _ =self.go_to_floor(signal).await;
                },
                Err(e) => {
                    print!("error receiving signal {}", e )
                }
            }
        }
    }
}

impl ElevatorControllerI for ElevatorController {
    async fn go_to_floor(&self, destination: usize) -> Result<(), Error> {
        let mut elevator = self.elevator.lock().await;

        _ = elevator.close_door().await;

        let mut previous_direction = elevator.direction.clone();

        if destination > elevator.current_floor {
            elevator.direction = "up".to_string()
        } else {
            elevator.direction = "down".to_string()
        }

        elevator.is_moving = true;
        let current_floor = elevator.current_floor;

        for i in current_floor + 1..destination + 1 {
            sleep(time::Duration::from_secs(1));
            elevator.current_floor = i;

            let ok = self.state_transmitter.send(
                ElevatorState {
                    id: elevator.id,
                    current_load: elevator.current_load,
                    direction:elevator.direction.clone(),
                    current_floor: elevator.current_floor,
                    previous_direction: previous_direction,
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

            previous_direction = elevator.direction.clone();
        }

        // self.destination_map.remove(&elevator.current_floor);
        elevator.is_moving = false;
        _ = elevator.open_door().await;

        // wait for 3 seconds before closing door
        sleep(time::Duration::from_secs(3));
        _ = elevator.close_door().await;

        if self.destination_list.len() == 0 as usize {
            let ok = self.state_transmitter.send(
                ElevatorState {
                    id: elevator.id,
                    current_load: elevator.current_load,
                    direction:"idle".to_string(),
                    current_floor: elevator.current_floor,
                    previous_direction: elevator.direction.clone(),
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
        }

        println!("done moving {}", elevator.id);

        Ok(())
    }
}
