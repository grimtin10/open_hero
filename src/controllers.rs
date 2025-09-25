use std::{collections::HashSet, sync::{mpsc::{self, Receiver}, Arc, Mutex}, thread, time::{Duration, Instant}};

use gilrs::{Axis, Button, Event, EventType, GamepadId, Gilrs};

macro_rules! push_event {
    ($event:expr, $sender:expr) => {
        if $sender.send($event).is_err() {
            break;
        }
    };
}

#[derive(Debug, Clone)]
pub enum ControllerEventType {
    ControllerConnected,
    ControllerDisconnected,
    ButtonPressed(Button),
    ButtonReleased(Button),
    AxisMotion { axis: Axis, value: f32 },
}

#[derive(Debug, Clone)]
pub struct ControllerEvent {
    pub controller: GamepadId,
    pub event: ControllerEventType,
    pub timestamp: u128,
}

pub struct ControllerManager {
    receiver: Receiver<ControllerEvent>,
    controllers: Arc<Mutex<HashSet<GamepadId>>>,
    #[allow(dead_code)]
    input_thread: thread::JoinHandle<()>,
    
    pub start: Instant,
}

impl ControllerManager {
    pub fn new() -> Result<Self, gilrs::Error> {
        let (sender, receiver) = mpsc::channel();
        let controllers = Arc::new(Mutex::new(HashSet::new()));
        let shared_controllers = controllers.clone();

        let mut gilrs = Gilrs::new()?;
        
        for (_id, gamepad) in gilrs.gamepads() {
            if gamepad.is_connected() {
                if let Ok(mut guard) = shared_controllers.lock() {
                    guard.insert(_id.into());
                }
            }
        }

        let start = Instant::now();

        let input_thread = thread::spawn(move || {
            println!("Controller input thread started.");

            loop {
                while let Some(Event { id, event, time, .. }) = gilrs.next_event() {
                    let timestamp = start.elapsed().as_micros();

                    match event {
                        EventType::Connected => {
                            println!("Controller {} connected", id);
                            
                            if let Ok(mut guard) = shared_controllers.lock() {
                                guard.insert(id);
                            }
                            push_event!(
                                ControllerEvent {
                                    controller: id,
                                    event: ControllerEventType::ControllerConnected,
                                    timestamp,
                                },
                                sender
                            );
                        }
                        EventType::Disconnected => {
                            println!("Controller {} disconnected", id);
                            
                            if let Ok(mut guard) = shared_controllers.lock() {
                                guard.remove(&id);
                            }
                            push_event!(
                                ControllerEvent {
                                    controller: id,
                                    event: ControllerEventType::ControllerDisconnected,
                                    timestamp,
                                },
                                sender
                            );
                        }
                        EventType::ButtonPressed(button, _code) => {
                            push_event!(
                                ControllerEvent {
                                    controller: id,
                                    event: ControllerEventType::ButtonPressed(button),
                                    timestamp,
                                },
                                sender
                            );
                        }
                        EventType::ButtonReleased(button, _code) => {
                            push_event!(
                                ControllerEvent {
                                    controller: id,
                                    event: ControllerEventType::ButtonReleased(button),
                                    timestamp,
                                },
                                sender
                            );
                        }
                        EventType::AxisChanged(axis, value, _code) => {
                            push_event!(
                                ControllerEvent {
                                    controller: id,
                                    event: ControllerEventType::AxisMotion { axis, value },
                                    timestamp,
                                },
                                sender
                            );
                        }
                        _ => {}
                    }
                }

                thread::sleep(Duration::from_micros(100));
            }
        });

        Ok(Self {
            receiver,
            controllers,
            input_thread,

            start,
        })
    }

    pub fn drain_events(&mut self) -> Vec<ControllerEvent> {
        self.receiver.try_iter().collect()
    }

    pub fn connected_controllers(&self) -> HashSet<GamepadId> {
        self.controllers.lock().unwrap().clone()
    }
}
