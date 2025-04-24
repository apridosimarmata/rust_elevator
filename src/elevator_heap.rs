
// Elevator is a max-heap, where the key is the current_load of each elevator
// calling a get() will return an elevator that is:
// 1. closest to the caller's floor
// 2. has least load (if it is not full yet)

//      48
//     /  \
//    21   35
//   /  \ /  \
// 17   6 12  1

use std::{collections::VecDeque, fmt::Error as StdError, pin::Pin};
use tokio::sync::Mutex;
use crate::interfaces::ElevatorHeapI;


#[derive(Debug)]
pub struct ElevatorHeap {
    elevators : VecDeque<MiniElevator>,
    mutation_permit: Mutex<bool>,
}


impl ElevatorHeapI for ElevatorHeap {
    async fn len(&self) -> usize{
        return self.elevators.len();
    }
    
    fn new() -> Self {
        ElevatorHeap { elevators: VecDeque::new(), mutation_permit:Mutex::new(true) }
    }
    
    // Get elevator receive floor where the call was made.
    // It will return a elevator based on these criteria:
    // 1. Least load
    async fn get_elevator(&mut self) -> usize {
        match self.elevators.pop_front() {
            Some(e) => {
                return e.id
            },
            None =>{
                return 0 as usize
            }
        }
    }

    async fn insert_elevator(&mut self, elevator: MiniElevator) -> Result<(), StdError> {
        let _ = self.mutation_permit.lock().await;
        self.elevators.push_back(MiniElevator { id: elevator.id, current_load: elevator.current_load, direction:elevator.direction });
        let _ = self.heapify(true, self.elevators.len()-1).await;

        Ok(())
    }

    async fn remove_elevator(&mut self, elevator_id: usize) -> Result<(), StdError> {

        let _ = self.mutation_permit.lock().await;

        let mut pos : usize = 0;
        let mut remove_at : usize = 0;
        let no_of_elevators = self.elevators.len();

        loop {
            if pos > no_of_elevators{
                break;
            }

            if self.elevators.get(pos).unwrap().id == elevator_id {
                remove_at = pos;
                break;
            }

            pos+=1;
        }

        if remove_at != no_of_elevators-1{
            self.elevators.swap(remove_at, no_of_elevators-1);
            self.elevators.pop_back();
            let _ = self.heapify(true, remove_at).await;
        }

        
        Ok(())
    }
}


#[derive(Debug, Clone)]
pub struct MiniElevator {
    pub id: usize,
    pub current_load: usize,
    pub direction: String,
}

// Assuming you have an Error type defined
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
    async fn heapify(&mut self, mutation_permitted: bool,index: usize) -> Result<(), MyError> {
        if index == 0 {
            return Ok(());
        }

        if mutation_permitted {
            let _ = self.mutation_permit.lock().await;
        }

        let parent_index = (index - 1)/2;

        let parent_elevator = self.elevators.get(parent_index);
        let target_elevator = self.elevators.get(index);

        if parent_elevator.is_some() && target_elevator.is_some(){
            if target_elevator.unwrap().current_load >= parent_elevator.unwrap().current_load {
                return Ok(())
            }

            // Swap
            self.elevators.swap(parent_index, index);

            let future = self.heapify(true, parent_index);
            let pinned_future: Pin<Box<dyn Future<Output = Result<(), MyError>>>> = Box::pin(future);

            return pinned_future.await;
        }


        Ok(())
    }
}