use std::time::Duration;

use crate::gcode_codec::{
    GCodeLine,
};

use nom_reprap_response::{
    Response,
};

use crate::protos::{
    machine_message,
    // MachineMessage,
    combinator_message,
    CombinatorMessage,
};

mod ready_state;
mod context;
mod effect;
mod send_serial;

mod task;
pub use task::Task;

pub use context::Context;
pub use effect::Effect;
pub use send_serial::send_serial;

mod disconnect;
pub use disconnect::disconnect;

use ready_state::ReadyState;

#[derive(Clone, Debug)]
pub enum Event {
    Init { serial_port_available: bool },
    ConnectionTimeout,
    GreetingTimerCompleted,
    SerialRec ((String, Response)),
    ProtobufClientConnection,
    ProtobufRec ( CombinatorMessage ),
    PollFeedback,
    TickleSerialPort,
    SerialPortDisconnected,
    SerialPortError{ message: String },
    GCodeLoaded(Task),
    GCodeLoadFailed{ task_id: u32, file_path: String },
}

#[derive(Clone, Debug)]
pub struct Connecting {
    baud_rate_candidates: Vec<u32>,
    received_greeting: bool,
}

#[derive(Clone, Debug)]
pub enum State {
    Disconnected,
    Connecting (Connecting),
    Ready ( ReadyState ),
    Errored { message: String },
    EStopped,
}

pub struct Loop {
    pub next_state: State,
    pub effects: Vec<Effect>,
}

impl Loop {
    fn new(next_state: State, effects: Vec<Effect>) -> Self {
        Self {
            next_state,
            effects,
        }
    }
}

use State::*;
use Event::*;

pub fn cancel_all_tasks(state: &State, context: &mut Context) {
    if let Ready( ReadyState { tasks, .. }) = state {
        tasks
            .iter()
            .for_each(|task| context.push_cancel_task(&task));
    };
}

fn errored(message: String, state: &State, context: &mut Context) -> Loop {
    error!("Error State: {:?}", message);

    if let Ready( ReadyState { tasks, .. }) = state {
        tasks
            .iter()
            .for_each(|task| {
                context.push_error(&task, &machine_message::Error { message: message.clone() });
            });
    };

    let next_state = Errored { message };
    context.handle_state_change(&next_state);

    let effects = vec![
        Effect::CancelAllDelays,
        Effect::ProtobufSend,
    ];

    Loop::new(next_state, effects)
}

fn append_to_error(message: String, next_line: &String, context: &mut Context) -> Loop {
    let effects = vec![
        Effect::CancelAllDelays,
        Effect::ProtobufSend,
    ];

    let message = format!("{}\n{}", message, next_line);

    let next_state = Errored { message };
    context.handle_state_change(&next_state);


    Loop::new(next_state, effects)
}

impl State {
    pub fn default_baud_rates() -> Vec<u32> {
        // baud rate candidates sorted by likelihood
        let mut baud_rates = vec![115_200, 250_000, 230_400, 57_600, 38_400, 19_200, 9_600];
        // Test order to see if baudrate detection works
        // let mut baud_rates = vec![250000, 230400, 57600, 38400, 19200, 9600, 115200];
        // let mut baud_rates = vec![115200];
        baud_rates.reverse();

        baud_rates
    }

    pub fn new_connection(baud_rate_candidates: Vec<u32>) -> Self {
        State::Connecting( Connecting { baud_rate_candidates, received_greeting: false })
    }

    fn and_no_effects(self) -> Loop {
        Loop {
            next_state: self,
            effects: vec![],
        }
    }

    fn invalid_transition_warning(self, event: &Event) -> Loop {
        warn!("Warning: received invalid event: {:?} in state: {:?}", event, self);

        self.and_no_effects()
    }

    fn invalid_transition_error(self, event: &Event, context: &mut Context) -> Loop {
        let message = format!("Invalid transition. State: {:?} Event: {:?}", self, event);

        errored(message, &self, context)
    }

    pub fn consume(self, event: Event, context: &mut Context) -> Loop {
        // eprintln!("event received {:?} in state {:?}", event, self);

        if let ProtobufClientConnection = &event {
            return Loop::new(
                self,
                vec![Effect::ProtobufSend],
            )
        }

        if let GCodeLoadFailed { file_path, ..} = &event {
            let message = format!("Failed to load GCode: {:}", file_path);
            return errored(message, &self, context)
        }


        if let ProtobufRec( CombinatorMessage { payload } ) = &event {
            use combinator_message::*;

            match payload {
                Some(Payload::DeviceDiscovered(_)) => {
                    info!("Device Discovered");
                    // Due to the async nature of discovery the new port could be discovered before disconnecting from the old one.
                    // The state machine will automatically attempt to reconnect on disconnect to handle this edge case.
                    return if let Disconnected = self {
                        self.reconnect_with_next_baud(context)
                    } else {
                        self.and_no_effects()
                    }
                }
                Some(Payload::DeviceDisconnected(_)) => {
                    // Due to the async nature of discovery the new port could be discovered before disconnecting from the old one.
                    // The state machine will automatically attempt to reconnect on disconnect to handle this edge case.
                    return if let Disconnected = self {
                        self.and_no_effects()
                    } else {
                        disconnect(&self, context)
                    }
                }
                Some(Payload::DeleteTaskHistory( DeleteTaskHistory { task_ids })) => {
                    context.delete_task_history(task_ids);
                    return self.and_no_effects()
                }
                Some(Payload::Estop(_)) => {
                    info!("ESTOP");

                    cancel_all_tasks(&self, context);

                    context.handle_state_change(&State::EStopped);

                    return Loop::new(
                        State::EStopped,
                        vec![
                            Effect::CloseSerialPort,
                            Effect::OpenSerialPort { baud_rate: 19_200 },
                            Effect::CancelAllDelays,
                            Effect::ProtobufSend,
                        ],
                    )
                }
                Some(Payload::Reset(_)) => {
                    info!("RESET: restarting service");

                    return Loop::new(
                        self,
                        vec![Effect::ExitProcess],
                    )
                }
                Some(Payload::ResetWhenIdle(_)) => {
                    let busy = if let Ready ( ready ) = &self {
                        ready.tasks.len() > 0
                    } else  {
                        false
                    };

                    return if busy {
                        context.reset_when_idle = true;

                        self.and_no_effects()
                    } else {
                        info!("RESET: restarting service");

                        Loop::new(
                            self,
                            vec![Effect::ExitProcess],
                        )
                    }
                }
                _ => ()
            }
        };
        
        if let Ready ( ready ) = self {
            ready.consume(event, context)
        } else {
            match event {
                Init { serial_port_available } => {
                    if serial_port_available {
                        info!("Teg Marlin: Started (Serial port found)");
                        self.reconnect_with_next_baud(context)
                    } else {
                        info!("Teg Marlin: Started (No device found)");
                        self.and_no_effects()
                    }
                }
                SerialPortDisconnected => {
                    disconnect(&self, context)
                    // eprintln!("Disconnected");
                    // Loop::new(
                    //     Disconnected,
                    //     vec![Effect::CancelAllDelays],
                    // )
                }
                SerialPortError { message } => {
                    error!("Disconnected due to serial port error: {:?}", message);
                    errored(message.to_string(), &self, context)
                }
                /* Echo, Debug and Error function the same in all states */
                SerialRec((src, response)) => {
                    context.push_gcode_rx(src.clone());

                    match (self, response) {
                        /* Errors */
                        (Errored { message }, _) => {
                            error!("RX ERR: {}", message);
                            append_to_error(message, &src, context)
                        }
                        (state, Response::Error(error)) => {
                            errored(error.to_string(), &state, context)
                        }
                        /* New socket */
                        (Connecting(conn @ Connecting { received_greeting: false, .. }), Response::Greeting) |
                        (Connecting(conn @ Connecting { received_greeting: false, .. }), Response::Ok {..}) => {
                            Self::receive_greeting(conn, &context)
                        }
                        /* Invalid transitions */
                        (state, response @ Response::Resend { .. }) => {
                            let event = SerialRec(( src, response ));
                            state.invalid_transition_error(&event, context)
                        }
                        /* No ops */
                        (state, _) => state.and_no_effects()
                    }
                }
                ConnectionTimeout => {
                    self.connection_timeout(event, context)
                }
                /* Awaiting Greeting Timer: After Delay */
                GreetingTimerCompleted => {
                    if let Connecting(Connecting { received_greeting: true, .. }) = self {
                        self.greeting_timer_completed(context)
                    } else {
                        self.invalid_transition_error(&event, context)
                    }
                },
                /* Warnings */
                PollFeedback |
                TickleSerialPort |
                GCodeLoaded(..) |
                GCodeLoadFailed{..} |
                ProtobufRec(_) |
                ProtobufClientConnection => {
                    self.invalid_transition_warning(&event)
                }
                /* Errors */
                // _ => self.invalid_transition_error(&event)
            }
        }
    }

    fn reconnect_with_next_baud(self, context: &mut Context) -> Loop {
        // let connection_timeout_ms = if let Connecting(_) = self {
        //     5_000
        // } else {
        //     1_000
        // };
        let connection_timeout_ms = context.controller.serial_connection_timeout;

        let new_connection = if let Connecting(Connecting { .. }) = self {
            false
        } else {
            true
        };

        let mut baud_rate_candidates = match &self {
            Connecting(Connecting { baud_rate_candidates, .. }) => baud_rate_candidates.clone(),
            _ => {
                let mut new_candidates = vec![context.controller.baud_rate];
                // prioritize the set baud rate in auto detection. That way we can cache the previous baud rate using
                // the baud_rate field. TODO: actually implement saving the previous baud rate
                if context.controller.automatic_baud_rate_detection {
                    new_candidates.extend(State::default_baud_rates());
                }
                new_candidates
            },
        };

        let baud_rate = baud_rate_candidates.pop();

        let mut effects = vec![
            Effect::CancelAllDelays,
        ];

        if let Some(baud_rate) = baud_rate {
            context.baud_rate = baud_rate;

            effects.append(&mut vec![
                Effect::OpenSerialPort { baud_rate },
                Effect::Delay {
                    key: "connection_timeout".to_string(),
                    duration: Duration::from_millis(connection_timeout_ms),
                    event: ConnectionTimeout,
                },
            ]);

            let mut next_state = Self::new_connection(baud_rate_candidates);

            // If the controller does not send a greeting then skip waiting for it
            if !context.controller.await_greeting_from_firmware {
                if let Connecting(connecting) = next_state {
                    let Loop {
                        next_state: after_greeting,
                        effects: mut greeting_effects,
                    } = Self::receive_greeting(connecting, &context);

                    next_state = after_greeting;
                    effects.append(&mut greeting_effects);
                } else {
                    panic!("Invariant: Connecting state not matched for new_connection")
                }
            }

            if new_connection {
                info!("Connecting to serial device...");
                context.handle_state_change(&next_state);
                effects.push(Effect::ProtobufSend);
            }

            Loop::new(
                next_state,
                effects,
            )
        } else {
            info!("Unable to Connect");

            disconnect(&self, context)
        }
    }

    fn connection_timeout(self, event: Event, context: &mut Context) -> Loop {
        if let Connecting(_) = self {
            self.reconnect_with_next_baud(context)
        } else {
            self.invalid_transition_error(&event, context)
        }
    }

    fn receive_greeting(mut connecting: Connecting, context: &Context) -> Loop {
        let delay = context.controller.delay_from_greeting_to_ready;
        info!("Greeting Received, waiting {}ms for firmware to finish startup", delay);

        let delay = Effect::Delay {
            key: "greeting_delay".to_string(),
            duration: Duration::from_millis(delay),
            event: GreetingTimerCompleted,
        };

        connecting.received_greeting = true;

        Loop::new(
            State::Connecting(connecting),
            vec![delay],
        )
    }

    fn greeting_timer_completed(self, context: &mut Context) -> Loop {
        let gcode = "M110 N0".to_string();

        let mut effects = vec![
            Effect::CancelAllDelays,
        ];

        send_serial(
            &mut effects,
            GCodeLine {
                gcode: gcode.clone(),
                line_number: None,
                checksum: true,
            },
            context,
        );

        let mut ready = ReadyState::default();
        ready.last_gcode_sent = Some(gcode);

        let next_state = Ready( ready );

        Loop::new(
            next_state,
            effects,
        )
    }
}
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn runs_the_greeting_handshake() {
//         let state = State::new_connection(State::default_baud_rates());
//         let event = SerialRec( ResponsePayload::Greeting );
//         let mut context = Context::new();
//
//         let Loop { next_state, effects } = state.consume(event, &mut context);
//
//         if let [Effect::Delay { event: GreetingTimerCompleted, .. }] = effects[..] {
//         } else {
//             panic!("Expected Delay, got: {:?}", effects)
//         };
//
//         if let Connecting(Connecting { received_greeting: true, .. }) = next_state {
//         } else {
//             panic!("Expected Delay, got: {:?}", next_state)
//         };
//     }
//
//     #[test]
//     fn ignores_multiple_greetings(context: &Context) {
//         let state = State::Connecting(Connecting {
//             baud_rate_candidates: State::default_baud_rates(),
//             received_greeting: true,
//         });
//         let event = SerialRec( ResponsePayload::Greeting );
//         let mut context = Context::new();
//
//         let Loop { next_state:_, effects } = state.clone().consume(event, &mut context);
//
//         assert!(effects.is_empty());
//         // TODO: equality checks
//         // assert_eq!(state, next_state);
//     }
//
//     #[test]
//     fn starts_the_printer_after_the_greeting_timer() {
//         let state = State::Connecting(Connecting {
//             baud_rate_candidates: State::default_baud_rates(),
//             received_greeting: true,
//         });
//         let event = GreetingTimerCompleted;
//         let mut context = Context::new();
//
//         let Loop { next_state, effects } = state.consume(event, &mut context);
//
//         match &effects[..] {
//             [Effect::SendSerial (GCodeLine { gcode, .. }), Effect::Delay {..}] if gcode[..] == *"M110 N0" => {}
//             _ => panic!("Expected SendSerial {{ gcode: \"M110 N0\" }}, got: {:?}", effects)
//         }
//
//         if let Ready { .. } = next_state {
//         } else {
//             panic!("Expected Ready, got: {:?}", next_state)
//         };
//     }
//
// }
