use core::time;
use std::{collections::{HashMap, VecDeque}, fmt::Error, thread::sleep};

use tokio::sync::broadcast::Sender;

use crate::interfaces::ElevatorI;

#[derive(Debug)]
pub struct Elevator {
    pub id: usize,

    // states
    pub is_door_open: bool,
    pub is_moving: bool,
    // is_in_maintenance: bool,

    pub current_floor: usize,
    // max_capacity: usize,
    pub current_load: usize,

    // destination_list: VecDeque<usize>,
    // destination_map: HashMap<usize, bool>,

    pub direction: String,
    // pub state_transmitter: Sender<ElevatorState>,
}

#[derive(Clone, Debug)]
pub struct ElevatorState {
    pub id: usize,

    pub current_floor: usize,
    pub current_load: usize,
    pub direction: String,
}

impl Elevator {
    pub fn new(id: usize) -> Elevator {
        let elevator = Elevator {
            id: id,
            is_door_open: false,
            is_moving: false,
            // is_in_maintenance: false,
            // destination_list: VecDeque::new(),
            current_floor: 0,
            // max_capacity: 15,
            current_load: 0,
            // destination_map: HashMap::new(),
            direction: "up".to_string(),
        };

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
}


impl ElevatorI for Elevator {
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

    // async fn move_to(&mut self, destination: usize) -> Result<(), Error>{
    //     for i in self.current_floor + 1..destination + 1 {
    //         self.current_floor = i;
            
    //         let ok = self.state_transmitter.send(
    //             ElevatorState {
    //                 id: self.id,
    //                 current_load: self.current_load,
    //                 direction:self.direction.clone(),
    //                 current_floor: self.current_floor
    //             }
    //         );

    //         match ok {
    //             Ok(_) => {
    //                 println!("{} moving to {} : now at {} published", self.id, destination, self.current_floor);
    //             },
    //             Err(e) => {
    //                 println!("got error on publishing elevator state {} {}", self.id, e);
    //             }
    //         }
    //         sleep(time::Duration::from_secs(1));
    //     }

    //     Ok(())
    // }
}