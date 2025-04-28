use std::{collections::HashMap, sync::Arc};

use futures::lock::Mutex;

use crate::{elevator::ElevatorState, interfaces::ElevatorPool};

use super::elevator_heap::MyError;

#[derive(Debug)]
pub struct ElevatorQueue {
    pub elevators: Arc<Mutex<Vec<ElevatorState>>>,
    pub elevators_index: Mutex<HashMap<usize, usize>>, // used to find an elevator quickly, or an elevator exists
}

impl ElevatorPool for ElevatorQueue  {
    async fn get_elevator_id(&mut self) -> Option<usize>{
        let elevators = self.elevators.lock().await;

        match elevators.last() {
            Some(e) => {
                return Some(e.id)
            },
            None => {
                return None;
            }
        }
    }

    fn new() -> Self {
        ElevatorQueue {
            elevators: Arc::new(Mutex::new(Vec::new())),
            elevators_index: Mutex::new(HashMap::new())
        }
    }

    async fn get_elevator(&mut self) -> Option<ElevatorState> {
        let mut elevators =  self.elevators.lock().await;
        let elevator =  elevators.pop();

        match elevator {
            Some(e) => {
                let mut elevator_index = self.elevators_index.lock().await;
                elevator_index.remove(&e.clone().id);

                return Some(e);
            },
            None => {

            }
        }

        return None;
    }

    async fn insert_elevator(&mut self, elevator: ElevatorState) -> Result<(), MyError> {
        let mut elevator_index = self.elevators_index.lock().await;

        match elevator_index.get(&elevator.id) {
            Some(e) => {
                return Ok(()) // no need to insert
            },
            None => {
            }
        }  

        let mut elevators= self.elevators.lock().await;
        let length  = elevators.len();

        elevators.insert(length, elevator);
        elevator_index.insert(length, 0);


        Ok(())
    }

    async fn remove_elevator(&mut self, elevator_id: usize) -> Option<ElevatorState> {
        let mut elevators =  self.elevators.lock().await;


        let mut elevator: Option<ElevatorState> = None;

        for e in elevators.clone().iter().enumerate() {
            if  e.1.id == elevator_id {
                let bind =  e.1.clone();
                elevator = Some(bind);
                elevators.remove(e.0);

                let mut elevator_index = self.elevators_index.lock().await;
                elevator_index.remove(&e.1.id);
                
                break;
            }
        }

        return elevator
    }

    async fn len(&self) -> usize {
        return self.elevators.lock().await.len();
    }
}