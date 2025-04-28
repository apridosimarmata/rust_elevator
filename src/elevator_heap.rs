// Elevator is a min-heap, where the key is the current_load of each elevator
// Calling a get_elevator() will return an elevator that with least load

//      1
//     /  \
//    2    5
//   /  \ /  \
//  7   6 9  10
use crate::{elevator_controller::ElevatorController, interfaces::ElevatorHeapI};
use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
    sync::Arc,
};
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct ElevatorHeap {
    elevators: Arc<Mutex<VecDeque<ElevatorController>>>,
    pub elevators_index: Arc<Mutex<HashMap<usize, usize>>>,
}

impl ElevatorHeapI for ElevatorHeap {
    async fn len(&self) -> usize {
        return self.elevators.lock().await.len();
    }

    fn new() -> Self {
        ElevatorHeap {
            elevators: Arc::new(Mutex::new(VecDeque::new())),
            elevators_index: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn get_elevator(&mut self) -> Option<ElevatorController> {
        match self.elevators.lock().await.pop_front() {
            Some(e) => return Some(e),
            None => return None,
        }
    }

    async fn insert_elevator(
        &mut self,
        elevator_controller: ElevatorController,
    ) -> Result<(), MyError> {


        Ok(())
    }

    async fn remove_elevator(&mut self, elevator_id: usize) -> Option<ElevatorController> {
 
        None
    }
}

#[derive(Debug)]
pub struct MyError {
    message: String,
}

impl std::fmt::Display for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for MyError {}

impl ElevatorHeap {
    async fn bubble_up(&mut self, index: usize) -> (usize, Option<MyError>) {


        return (
            0,
            Some(MyError {
                message: "parent is missing".to_string(),
            }),
        );
    }

    async fn bubble_down(&mut self, index: usize) -> (usize, Option<MyError>) {

        return (self.elevators.lock().await.len() - 1, None);
    }
}
