use std::{collections::{HashMap, VecDeque}, sync::Arc};
use crate::{elevator::ElevatorState, interfaces::ElevatorPool};
use super::elevator_heap::MyError;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct ElevatorQueue {
    pub elevators: Arc<Mutex<VecDeque<ElevatorState>>>,
    pub elevators_index: Mutex<HashMap<usize, usize>>, // used to find an elevator quickly, or an elevator exists
}

impl ElevatorPool for ElevatorQueue  {
    fn new() -> Self {
        ElevatorQueue {
            elevators: Arc::new(Mutex::new(VecDeque::new())),
            elevators_index: Mutex::new(HashMap::new())
        }
    }

    async fn get_elevator(&mut self) -> Option<ElevatorState> {
        let mut elevators =  self.elevators.lock().await;
        let elevator =  elevators.pop_back();

        match elevator {
            Some(e) => {
                let mut elevator_index = self.elevators_index.lock().await;
                elevator_index.remove(&e.clone().id);

                return Some(e);
            },
            None => {
                return None;
            }
        }
    }

    async fn insert_elevator(&mut self, elevator: ElevatorState) -> Result<(), MyError> {
        let mut elevator_index = self.elevators_index.lock().await;

        match elevator_index.get(&elevator.id) {
            Some(_) => {
                return Ok(())
            },
            None => {
                let mut elevators= self.elevators.lock().await;
                elevator_index.insert(elevator.id, 0);
                elevators.push_front(elevator);
        
                Ok(())
            }
        }  


    }

    async fn remove_elevator(&mut self, elevator_id: usize) -> Option<ElevatorState> {
        let mut elevators =  self.elevators.lock().await;
        let mut elevator_index = self.elevators_index.lock().await;

        match elevator_index.get(&elevator_id) {
            Some(e) => {
                let index = *e;

                elevator_index.remove(&elevator_id);
                match elevators.remove(index){
                    Some(e) => {
                        return Some(e)
                    },
                    None => {
                        return None;
                    }
                }
            },
            None => {
                return None
            }
        }  

    }

    async fn len(&self) -> usize {
        return self.elevators.lock().await.len();
    }
}