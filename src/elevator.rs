use core::time;
use std::{fmt::Error, thread::sleep};


use serde::{Deserialize, Serialize};

use crate::interfaces::ElevatorI;

#[derive(Debug)]
pub struct Elevator {
    pub id: usize,

    // states
    pub is_door_open: bool,
    pub is_moving: bool,

    pub current_floor: usize,
    pub current_load: usize,

    pub direction: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ElevatorState {
    pub id: usize,
    pub current_floor: usize,
    pub current_load: usize,
    pub direction: String,
    pub initial_direction: String,
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
            direction: "idle".to_string(),
        };

        return elevator;
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
}