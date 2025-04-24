use std::fmt::Error;

use crate::{elevator::{Elevator, ElevatorState}, elevator_controller::ElevatorController, elevator_heap::MiniElevator};


// Used by elevator controller
pub trait ElevatorHeapI {
     fn new() -> Self;
     async fn get_elevator(&mut self) -> Option<ElevatorController>;
     async fn insert_elevator(&mut self, elevator: ElevatorController) -> Result<(), Error>;
     async fn remove_elevator(&mut self, elevator_id: usize) -> Result<(), Error>;
     async fn len(&self) -> usize;

}

pub trait ElevatorControllerI {
    async fn go_to_floor(&self, destination: usize) -> Result<(), Error>;

    // async fn add_destination(&mut self, floor: usize) -> Result<(), Error>;
}

pub trait CentralElevatorControllerI {
    async fn call_for_an_elevator(&self, floor: usize, destination: String) -> Result<(), Error>;
}

pub trait ElevatorI {
    async fn close_door(&mut self) -> Result<(), Error>;
    async fn open_door(&mut self) -> Result<(), Error>;
    // async fn move_to(&mut self, destination: usize) -> Result<(), Error>;
}