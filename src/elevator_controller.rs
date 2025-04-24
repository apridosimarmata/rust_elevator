use std::{collections::HashMap, fmt::Error, sync::Arc};

use tokio::sync::broadcast::Sender;
use tokio::sync::Mutex;
use tokio::sync::broadcast::{Receiver, channel};
use crate::elevator::ElevatorState;
use crate::elevator_heap::MiniElevator;
use crate::{
    elevator::Elevator,
    elevator_heap::ElevatorHeap,
    interfaces::{ElevatorControllerI, ElevatorHeapI, ElevatorI},
};

pub struct ElevatorController {
    moving_up_elevators: Mutex<ElevatorHeap>,
    moving_down_elevators: Mutex<ElevatorHeap>,
    idle_elevators: Mutex<ElevatorHeap>,
    elevators: HashMap<usize, Arc<Mutex<Elevator>>>,
}

impl ElevatorController {
    pub async fn new(
    ) -> Self {
        let mut elevators :HashMap<usize, Arc<Mutex<Elevator>>> = HashMap::new();

        for i in 0..2 {
            let (state_tx, mut state_rx): (Sender<ElevatorState>, Receiver<ElevatorState>) = channel(100);
            
            let shared = Arc::new(Mutex::new(Elevator::new(i, state_tx.clone())));
            elevators.insert(
                i as usize, 
                shared
            );

            tokio::spawn(async move {
                loop{
                    println!("waiting for a new state!");
                    match state_rx.recv().await{
                        Ok(state) => {
                            println!("elevator {}: is at {} floor with current load {} and moving {}", state.id, state.current_floor, state.current_load, state.direction);
                        },
                        Err(e) => {
    
                        }
                    }
                }
            });
        }

        let idle_elevators = Mutex::new(ElevatorHeap::new());
        let mut guard = idle_elevators.lock().await;



        for e in elevators.clone() {
            let elevator_guard = e.1.lock().await;
            let _ = guard.insert_elevator(MiniElevator{
                id: elevator_guard.id,
                current_load: elevator_guard.current_load,
                direction: elevator_guard.direction.clone(),

            }).await;
        }

        drop(guard);

        let  controller = ElevatorController {
            elevators: elevators,
            moving_down_elevators: Mutex::new(ElevatorHeap::new()),
            moving_up_elevators: Mutex::new(ElevatorHeap::new()),
            idle_elevators: idle_elevators,
        };



        return controller
    }

}

impl ElevatorControllerI for ElevatorController {
    async fn get_elevators_state(&self) -> Vec<ElevatorState> {
        let mut elevators: Vec<ElevatorState> = Vec::new();

        let elevators_state = self.elevators.clone();
        for item in elevators_state {
            let elevator = item.1.lock().await;
            elevators.push(elevator.get_state());
        }

        return elevators;
    }

    async fn call_for_an_elevator(&self, floor: usize, direction: String) -> Result<(), Error> {
        let elevator_id: usize;

        // check if any idle elevator available
        let idle_elevators = self.idle_elevators.lock().await;
        let idles = idle_elevators.len().await;
        drop(idle_elevators);

        if idles > 0 {
            // choose idle elevator
            let mut idle_elevators = self.idle_elevators.lock().await;
            elevator_id = idle_elevators.get_elevator().await;
            drop(idle_elevators);
        } else if direction == "up".to_string() {
            // choose elevator that is moving up
            let mut moving_up_elevators = self.moving_up_elevators.lock().await;
            elevator_id = moving_up_elevators.get_elevator().await;
            drop(moving_up_elevators);
        } else {
            // choose elevator that is moving down
            let mut moving_down_elevators = self.moving_down_elevators.lock().await;
            elevator_id = moving_down_elevators.get_elevator().await;
            drop(moving_down_elevators);
        }

        let elevator = self.elevators.get(&elevator_id);
        match elevator {
            // send request to selected elevator
            Some(e) => {
                let bind = e.clone();
                let mut elevator = bind.lock().await;
                let _ = elevator.add_destination(floor).await;
                drop(elevator);
                return Ok(());
            }
            None => {}
        }


        Ok(())
    }
}
