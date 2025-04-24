use std::{
    collections::{HashMap, VecDeque}, fmt::Error, thread::sleep, time
};

use tokio::sync::broadcast::{Receiver, Sender, channel};
use crate::interfaces::ElevatorI;

#[derive(Clone)]
pub struct Elevator {
    pub id: usize,

    // states
    is_door_open: bool,
    is_moving: bool,
    is_in_maintenance: bool,

    pub current_floor: usize,
    max_capacity: usize,
    pub current_load: usize,

    destination_list: VecDeque<usize>,
    destination_map: HashMap<usize, bool>,

    pub direction: String,

    signal_transmitter: Sender<usize>,
    pub state_transmitter: Sender<ElevatorState>
}

#[derive(Clone)]
pub struct ElevatorState {
    pub id: usize,

    pub current_floor: usize,
    pub current_load: usize,
    pub direction: String,
}

impl Elevator {
    pub fn new(id: usize, state_transmitter: Sender<ElevatorState> ) -> Elevator {
        let (tx, rx): (Sender<usize>, Receiver<usize>) = channel(10);


        let elevator = Elevator {
            id: id,
            is_door_open: false,
            is_moving: false,
            is_in_maintenance: false,
            destination_list: VecDeque::new(),
            current_floor: 0,
            max_capacity: 15,
            current_load: 0,
            destination_map: HashMap::new(),
            direction: "up".to_string(),
            signal_transmitter: tx,
            state_transmitter: state_transmitter,
        };

        let mut cloned_elevator = elevator.clone();

        tokio::spawn(async move {
            cloned_elevator.run(rx).await;
        });

        return elevator;
    }

    pub fn get_state(&self) -> ElevatorState {
        return ElevatorState {
            id: self.id,
            current_floor: self.current_floor,
            current_load: self.current_load,
            direction: self.direction.clone(),
        };
    }

    pub async fn run(&mut self, mut signal_receiver: Receiver<usize>) {
        loop {
            // instead of thread::sleep(), wait for a signal
            let signal = signal_receiver.recv().await;

            match signal {
                Ok(s) => {
                    self.destination_list.push_front(s);
                }
                Err(e) => {
                    // got error, wait for next one
                    println!("{:?}", e.to_string());
                    continue;
                }
            }

            match self.destination_list.pop_back() {
                Some(destination) => {
                    let _ = self.go_to_floor(destination).await;
                }
                None => {
                    continue;
                }
            }
        }
    }
}

impl ElevatorI for Elevator {
    async fn go_to_floor(&mut self, destination: usize) -> Result<(), Error> {
        println!("{} moving to {}", self.id, destination);

        _ = self.close_door().await;

        if destination > self.current_floor {
            self.direction = "up".to_string()
        } else {
            self.direction = "down".to_string()
        }

        self.is_moving = true;

        for i in self.current_floor + 1..destination + 1 {
            self.current_floor = i;
            println!("{} moving to {} : now at {}", self.id, destination, self.current_floor);
            
            let ok = self.state_transmitter.send(
                ElevatorState {
                    id: self.id,
                    current_load: self.current_load,
                    direction:self.direction.clone(),
                    current_floor: self.current_floor
                }
            );

            match ok {
                Ok(_) => {
                    println!("{} moving to {} : now at {} published", self.id, destination, self.current_floor);
                },
                Err(e) => {
                    println!("{} moving to {} : now at {} error publishing", self.id, destination, self.current_floor);
                }
            }
            sleep(time::Duration::from_secs(1));

        }

        self.destination_map.remove(&self.current_floor);
        self.is_moving = false;
        _ = self.open_door().await;

        // wait for 3 seconds before closing door
        _ = self.close_door().await;

        if self.destination_list.len() == 0 as usize {
            self.direction = "idle".to_string();   
        }

        Ok(())
    }

    async fn add_destination(&mut self, floor: usize) -> Result<(), Error> {
        // check is floor already in destination
        match self.destination_map.get(&floor) {
            Some(f) => {
                if *f == true {
                    return Ok(());
                }
            }
            None => {}
        }

        // no need concurrent handling
        let result = self.signal_transmitter.send(floor);
        match result {
            Ok(_) => {

            },
            Err(e) =>{
                println!("failed {:?}", self.destination_list);
            }
        }
        Ok(())
    }

    async fn close_door(&mut self) -> Result<(), Error> {
        sleep(time::Duration::from_secs(1));
        self.is_door_open = false;
        Ok(())
    }

    async fn open_door(&mut self) -> Result<(), Error> {
        sleep(time::Duration::from_secs(1));
        self.is_door_open = true;
        Ok(())
    }
}
