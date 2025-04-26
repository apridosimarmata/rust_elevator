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
        let elevator_id = elevator_controller.elevator_id;

        let mut elevators_index = self.elevators_index.lock().await;
        let elevator_index_bind = elevators_index
            .get(&elevator_id.clone())
            .clone();

        println!("elevator index in heap {:?}", elevator_index_bind);
        println!("padahal {:?}", elevators_index);

        if elevator_index_bind.is_some() {
            let mut bind = *elevator_index_bind.unwrap();
            drop(elevators_index);

            /* heapify up */
            loop {
                if bind == 0 {
                    break;
                }

                let result = self.move_up(bind).await;
                if result.1.is_some() {
                    return Err(MyError {
                        message: " ok".to_string(),
                    });
                } else {
                    bind = result.0
                }
            }
            /* end of heapify up */
        } else {
            self.elevators.lock().await.push_back(elevator_controller);
            elevators_index.insert(elevator_id, self.elevators.lock().await.len() - 1);
            drop(elevators_index);
            println!("{:?}", self.elevators_index.lock().await);
        }

        tokio::task::yield_now().await;

        Ok(())
    }

    async fn remove_elevator(&mut self, elevator_id: usize) -> Option<ElevatorController> {
        let elevator_controller: Option<ElevatorController> = None;

        let mut elevators_index = self.elevators_index.lock().await;
        let mut elevators = self.elevators.lock().await;

        let index = elevators_index.get(&elevator_id);

        if index.is_none() {
            return None;
        }

        let mut index_bind = *index.unwrap();

        // remove the node
        if index_bind == 0{
            elevators_index.remove(&elevator_id);
            return elevators.pop_back();
        }
        dbg!("swapping: {} ", index_bind);

        elevators.swap(index_bind, elevators_index.len() - 1);
        elevators.pop_back();

        let elevators_length = elevators.len();
        elevators_index.remove(&elevator_id);
        drop(elevators_index);
        drop(elevators);

        /* heapify down */
        loop {
            dbg!("here");
            if index_bind == elevators_length {
                break;
            }

            let result = self.move_down(index_bind).await;
            if result.1.is_some() {
                return None;
            } else {
                index_bind = result.0
            }
        }
        /* end of heapify down */



        elevator_controller
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
    async fn move_up(&mut self, index: usize) -> (usize, Option<MyError>) {
        if index == 0 {
            return (0, None);
        }

        let parent_index = (index - 1) / 2;

        let elevators = self.elevators.lock().await;

        let parent = elevators.get(parent_index);
        let target = elevators.get(index);

        if parent.is_some() && target.is_some() {
            let parent_elevator_controller = parent.unwrap();
            let target_elevator_controller = target.unwrap();

            let parent_elevator = parent_elevator_controller.elevator.lock().await;
            let target_elevator = target_elevator_controller.elevator.lock().await;

            if target_elevator.current_load >= parent_elevator.current_load {
                return (0, None);
            }

            let parent_elevator_id = parent_elevator.id;
            let target_elevator_id = target_elevator.id;

            drop(parent_elevator);
            drop(target_elevator);

            // Swap
            self.elevators.lock().await.swap(parent_index, index);

            let mut elevators_index = self.elevators_index.lock().await;

            println!("mutating?");
            elevators_index.insert(parent_elevator_id, index);
            elevators_index.insert(target_elevator_id, parent_index);
            println!("mutated");

            drop(elevators_index);
            drop(elevators);

            tokio::task::yield_now().await;
            return (parent_index, None);
        }

        return (
            0,
            Some(MyError {
                message: "parent is missing".to_string(),
            }),
        );
    }

    async fn move_down(&mut self, index: usize) -> (usize, Option<MyError>) {
        /* check if target has children */
        let left_child_index = (index * 2) + 1;
        let right_child_index = (index * 2) + 2;

        let mut elevators = self.elevators.lock().await;
        let mut elevators_index = self.elevators_index.lock().await;

        if left_child_index >= elevators.len() {
            return (self.elevators.lock().await.len(), None);
        }

        /* === has at least one child === */

        /* target elevator */
        let target_node = elevators.get(index);
        if target_node.is_none() {
            return (
                0,
                Some(MyError {
                    message: " ok".to_string(),
                }),
            );
        }
        let target_bind = target_node.unwrap();
        let target_elevator = target_bind.elevator.lock().await;
        /* end of target elevator */

        /* left elevator */
        let left_child_node = elevators.get(left_child_index);
        if left_child_node.is_none() {
            return (
                0,
                Some(MyError {
                    message: " ok".to_string(),
                }),
            );
        }
        let left_bind = left_child_node.unwrap();
        let left_elevator = left_bind.elevator.lock().await;
        /* end of left elevator */

        if right_child_index >= elevators.len() {
            // right child node does not exists
            if left_elevator.current_load > target_elevator.current_load {
                // Swap
                elevators_index.insert(left_elevator.id, index);
                elevators_index.insert(target_elevator.id, left_child_index);

                drop(left_elevator);
                drop(target_elevator);
                elevators.swap(index, left_child_index);
            }

            return (left_child_index, None);
        }

        /* both right & left child exist */
        let right_child_node = elevators.get(right_child_index);
        if right_child_node.is_none() {
            return (
                0,
                Some(MyError {
                    message: " ok".to_string(),
                }),
            );
        }
        let right_bind = right_child_node.unwrap();
        let right_elevator = right_bind.elevator.lock().await;

        /* select max loaded elevator */
        let mut max_child_index = left_child_index;
        let mut max_load = left_elevator.current_load;
        let mut max_id = left_elevator.id;

        if max_load < right_elevator.current_load {
            max_child_index = right_child_index;
            max_load = right_elevator.current_load;
            max_id = right_elevator.id;
        }

        if target_elevator.current_load < max_load {
            // Swap
            elevators_index.insert(max_id, index);
            elevators_index.insert(target_elevator.id, max_child_index);

            drop(left_elevator);
            drop(right_elevator);
            drop(target_elevator);
            elevators.swap(index, max_child_index);

            return (max_child_index, None);
        }

        // stop process
        return (elevators.len() - 1, None);
    }
}
