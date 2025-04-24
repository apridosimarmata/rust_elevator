use std::collections::VecDeque;
use std::{collections::HashMap, fmt::Error, sync::Arc};

use tokio::sync::broadcast::Sender;
use tokio::sync::Mutex;
use tokio::sync::broadcast::{Receiver, channel};
use crate::elevator::ElevatorState;
use crate::elevator_controller::ElevatorController;
use crate::interfaces::CentralElevatorControllerI;
use crate::{
    elevator::Elevator,
    elevator_heap::ElevatorHeap,
    interfaces::ElevatorHeapI,
};

pub struct CentralElevatorController {
    moving_up_elevators: Mutex<ElevatorHeap>,
    moving_down_elevators: Mutex<ElevatorHeap>,
    idle_elevators: Mutex<ElevatorHeap>,
    signal_transmitter: HashMap<usize, Sender<usize>>,
}

impl CentralElevatorController {
    pub async fn new(
    ) -> Self {
        let mut idle_elevators = ElevatorHeap::new();
        let mut signal_transmitter: HashMap<usize, Sender<usize>>=  HashMap::new();

        /* Building elevators */
        for i in 0..2 {
            let (state_tx, mut state_rx): (Sender<ElevatorState>, Receiver<ElevatorState>) = channel(1);
            let shared = Arc::new(Mutex::new(Elevator::new(i)));


            /* Spawning state receivers for each elevator */
            tokio::spawn(async move {
                loop{
                    match state_rx.recv().await{
                        Ok(state) => {
                            println!("elevator {}: is at {} floor with current load {} and moving {}", state.id, state.current_floor, state.current_load, state.direction);
                        },
                        Err(e) => {
                            println!("got error when receiving elevator state {}", e);
                        }
                    }
                }
            });

            let (signal_tx, signal_rx): (Sender<usize>, Receiver<usize>) = channel(1);
            signal_transmitter.insert(i, signal_tx);

            /* Put the elevator to idles elevator */
            let elevator_controller = ElevatorController{
                elevator: shared,
                destination_map: HashMap::new(),
                destination_list: VecDeque::new(),
                state_transmitter: state_tx,
            };

            /* Runner for receiving requests from central controller */
            let runner_controller_bind = elevator_controller.clone();
            tokio::spawn(async move {
                runner_controller_bind.run(signal_rx).await;
            });

            let _ = idle_elevators.insert_elevator(elevator_controller).await;

        }

        let  controller = CentralElevatorController {
            moving_down_elevators: Mutex::new(ElevatorHeap::new()),
            moving_up_elevators: Mutex::new(ElevatorHeap::new()),
            idle_elevators: Mutex::new(idle_elevators),
            signal_transmitter: signal_transmitter,
        };

        return controller
    }

}

impl CentralElevatorControllerI for CentralElevatorController {

    async fn call_for_an_elevator(&self, floor: usize, direction: String) -> Result<(), Error> {
        let elevator: Option<ElevatorController>;

        /* Check if any idle elevator available */
        let idle_elevators = self.idle_elevators.lock().await;
        let idles = idle_elevators.len().await;
        drop(idle_elevators);

        if idles > 0 {
            /* Choose idle elevator */
            let mut idle_elevators = self.idle_elevators.lock().await;
            elevator = idle_elevators.get_elevator().await;
            drop(idle_elevators);
        } else if direction == "up".to_string() {
            /* Choose elevator that is moving up */
            let mut moving_up_elevators = self.moving_up_elevators.lock().await;
            elevator = moving_up_elevators.get_elevator().await;
            drop(moving_up_elevators);
        } else {
            /* Choose elevator that is moving down */
            let mut moving_down_elevators = self.moving_down_elevators.lock().await;
            elevator = moving_down_elevators.get_elevator().await;
            drop(moving_down_elevators);
        }

        match elevator {
            // send request to selected elevator
            Some(e) => {
                let elevator = e.elevator.lock().await;
                let elevator_id = elevator.id;
                drop(elevator);

                let transmitter = self.signal_transmitter.get(&elevator_id);
                match transmitter {
                    Some(t) => {
                        let _ = t.send(floor);
                    },
                    None => {

                    },
                }

                return Ok(())
            }
            None => {}
        }


        Ok(())
    }
}
