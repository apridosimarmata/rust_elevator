use std::fmt::Error;

use crate::{elevator_controller::ElevatorController, elevator_heap::MyError};

pub trait ElevatorHeapI {
     fn new() -> Self;
     async fn get_elevator(&mut self) -> Option<ElevatorController>;
     async fn insert_elevator(&mut self, elevator: ElevatorController) -> Result<(), MyError>;
     async fn remove_elevator(&mut self, elevator_id: usize) -> Option<ElevatorController>;
     async fn len(&self) -> usize;

}

pub trait ElevatorControllerI {
    async fn go_to_floor(&self, destination: usize) -> Result<(), Error>;
}

pub trait CentralElevatorControllerI {
    async fn call_for_an_elevator(&self, floor: usize, destination: String) -> Result<(), Error>;
}

pub trait ElevatorI {
    async fn close_door(&mut self) -> Result<(), Error>;
    async fn open_door(&mut self) -> Result<(), Error>;
}