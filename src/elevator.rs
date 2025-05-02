use tokio::time::{sleep, Duration};
use std::fmt::Error;


use serde::{Deserialize, Serialize};

use crate::interfaces::ElevatorI;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElevatorState {
    pub id: usize,

    // states
    pub is_door_open: bool,
    pub is_moving: bool,

    pub current_floor: usize,
    pub current_load: usize,

    pub direction: String,
    pub initial_direction: String,
}

impl ElevatorState {
    pub fn new(id: usize) -> ElevatorState {
        let elevator = ElevatorState {
            id: id,
            is_door_open: false,
            is_moving: false,
            current_floor: 0,
            current_load: 0,
            direction: "idle".to_string(),
            initial_direction: "idle".to_string(),
        };

        return elevator;
    }
}


impl ElevatorI for ElevatorState {
     async fn close_door(&mut self) -> Result<(), Error> {
        sleep(Duration::from_secs(1)).await;
        self.is_door_open = false;
        Ok(())
    }
    
    async fn open_door(&mut self) -> Result<(), Error> {
        sleep(Duration::from_secs(1)).await;
        self.is_door_open = true;
        Ok(())
    }
}