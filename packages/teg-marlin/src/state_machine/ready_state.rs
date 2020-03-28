use std::time::Duration;

use crate::gcode_codec::{
    GCodeLine,
    response::{
        ResponsePayload,
        Feedback,
    }
};

use super::{
    Loop,
    State::*,
    Event::{self, *},
    Effect,
    Task,
    errored,
    send_serial,
    Context,
    disconnect,
};

use crate::protos::{
    // MachineMessage,
    // machine_message,
    CombinatorMessage,
    combinator_message,
};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Polling {
    PollTemperature,
    PollPosition,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum OnOK {
    TransitionToReady,
    Resend,
    IgnoreOK,
    Despool,
    NotAwaitingOk,
}

#[derive(Clone, Debug)]
pub struct ReadyState {
    poll_for: Option<Polling>,
    awaiting_polling_delay: bool,
    tickles_attempted: u32,
    pub last_gcode_sent: Option<String>,
    on_ok: OnOK,
    next_serial_line_number: u32,
    // spool
    loading_gcode: bool,
    pub task: Option<Task>,
}

impl Default for ReadyState {
    fn default() -> Self {
        Self {
            on_ok: OnOK::TransitionToReady,
            last_gcode_sent: None,
            poll_for: Some(Polling::PollPosition),
            awaiting_polling_delay: false,
            tickles_attempted: 0,
            next_serial_line_number: 1,
            // spool
            loading_gcode: false,
            task: None,
        }
    }
}

impl ReadyState {
    fn and_no_effects(self) -> Loop {
        Ready( self ).and_no_effects()
    }

    pub fn consume(mut self, event: Event, context: &mut Context) -> Loop {
        // eprintln!("E: {:?}", event);
        match event {
            ProtobufRec( msg@CombinatorMessage { payload: None } ) => {
                eprintln!("Warning: CombinatorMessage received without a payload. Ignoring: {:?}", msg);
                self.and_no_effects()
            }
            ProtobufRec( CombinatorMessage { payload: Some(message) } ) => {
                // eprintln!("PROTOBUF RECEIVED WHEN READY {:?}", message);
                match message {
                    combinator_message::Payload::SpoolTask(spool_task) => {
                        use combinator_message::{
                            spool_task::Content,
                            InlineContent,
                        };

                        self.loading_gcode = true;

                        let combinator_message::SpoolTask {
                            task_id,
                            client_id,
                            content,
                            ..
                        } = spool_task;

                        match content {
                            Some(Content::Inline ( InlineContent { commands })) => {
                                let task = Task {
                                    id: spool_task.task_id,
                                    client_id,
                                    gcode_lines: commands.into_iter(),
                                };
                                self.consume(GCodeLoaded(task), context)
                            }
                            Some(Content::FilePath( file_path )) => {
                                let effects = vec![
                                    Effect::LoadGCode {
                                        file_path: file_path.clone(),
                                        task_id,
                                        client_id
                                    },
                                ];
                                Loop::new(Ready(self), effects)
                            }
                            None => {
                                eprintln!("Warning: spool_task received without content. Ignoring.");
                                self.and_no_effects()
                            }
                        }
                    }
                    combinator_message::Payload::PauseTask(combinator_message::PauseTask { task_id }) => {
                        self.loading_gcode = true;
                        match self.task.as_ref() {
                            Some(task) if task.id == task_id => {
                                context.push_pause_task(&task);
                                self.task = None;
                                context.feedback.despooled_line_number = 0;

                                if context.reset_when_idle {
                                    return Loop::new(
                                        Ready(self),
                                        vec![Effect::ExitProcessAfterDelay],
                                    )
                                }

                                self.and_no_effects()
                            }
                            _ => {
                                self.and_no_effects()
                            }
                        }
                    }
                    _ => {
                        self.and_no_effects()
                    }
                }
            }
            SerialPortDisconnected => {
                disconnect(context)
            }
            GCodeLoaded ( task ) => {
                // eprintln!("LOADED {:?}", self.on_ok);
                context.push_start_task(&task);

                // despool the first line of the task if ready to do so
                if let OnOK::NotAwaitingOk = self.on_ok {
                    let mut effects = vec![];

                    self.task = Some(task);
                    self.despool_task(&mut effects, context);

                    Loop::new(Ready(self), effects)
                } else {
                    self.task = Some(task);
                    self.and_no_effects()
                }
            }
            /* Echo, Debug and Error function the same in all states */
            SerialRec( response ) => {
                if let Some(_) = &self.task {
                    context.push_gcode_rx(response.raw_src);
                }

                match response.payload {
                    /* No ops */
                    ResponsePayload::Echo |
                    ResponsePayload::Debug |
                    ResponsePayload::Warning { .. } => {
                        self.and_no_effects()
                    }
                    /* Errors */
                    ResponsePayload::Error(error) => {
                        errored(error.to_string(), context)
                    }
                    ResponsePayload::Greeting => {
                        let message = format!("Unexpected printer firmware restart. State: {:?}", self);

                        errored(message, context)
                    }
                    ResponsePayload::Ok( feedback ) => {
                        let mut effects = self.receive_feedback( &feedback, context );
                        self.receive_ok(&mut effects, context);

                        Loop::new(
                            Ready(self),
                            effects,
                        )
                    }
                    ResponsePayload::Feedback( feedback ) => {
                        let mut effects = self.receive_feedback( &feedback, context );
                        effects.push(
                            Effect::CancelDelay { key: "tickle_delay".to_string() }
                        );

                        Loop::new(
                            Ready(self),
                            effects,
                        )
                    }
                    ResponsePayload::Resend { line_number } => {
                        self.receive_resend_request(line_number, context)
                    }
                }
            }
            PollFeedback => {
                self.poll_for = Some(Polling::PollPosition);

                // eprintln!("POLL FEEDBACK!!!!! {:?}", self);
                if let OnOK::NotAwaitingOk = self.on_ok {
                    let mut effects = vec![];

                    self.poll_feedback(&mut effects, context, Polling::PollPosition);

                    Loop::new(
                        Ready(self),
                        effects,
                    )
                } else {
                    self.and_no_effects()
                }
            }
            TickleSerialPort => {
                self.tickle_serial_port(context)
            }
            _ => Ready( self ).invalid_transition_error(&event, context)
        }
    }

    fn receive_feedback(&mut self, feedback: &Feedback, context: &mut Context) -> Vec<Effect> {
        let mut effects = vec![];

        // set actual_temperatures
        for (address, val) in feedback.actual_temperatures.iter() {
            if address == "E" {
                // Skip "E" values. I have no idea what they are for but Marlin always sends them.
                continue
            } if let Some(heater) = context.feedback.heaters.iter_mut().find(|h| h.address == *address) {
                heater.actual_temperature = *val;
            }
            else {
                eprintln!("Warning: unknown actual_temperature address: {:?} = {:?}°C", address, val);
            }
        }

        // set actual_positions
        for (address, val) in feedback.actual_positions.iter() {
            if let Some(heater) = context.feedback.axes.iter_mut().find(|h| h.address == *address) {
                heater.actual_position = *val;
            }
            else {
                eprintln!("Warning: unknown actual_position axis: {:?} = ${:?}", address, val);
            }
        }

        // update polling timers and send to protobuf clients
        if feedback.actual_temperatures.len() != 0 {
            let delay = Effect::Delay {
                key: "polling_delay".to_string(),
                duration: Duration::from_millis(context.controller.polling_interval),
                event: PollFeedback,
            };

            self.awaiting_polling_delay = true;

            effects.push(delay);
            effects.push(Effect::ProtobufSend);
        }

        effects
    }

    fn receive_ok(&mut self, effects: &mut Vec<Effect>, context: &mut Context) {
        match self.on_ok {
            OnOK::NotAwaitingOk => {
                // Ignore erroneous OKs
            }
            OnOK::Resend => {
                let gcode = self.last_gcode_sent
                    .clone()
                    .expect("Cannot resend GCode if none was sent");

                send_serial(
                    effects,
                    GCodeLine {
                        gcode,
                        line_number: Some(self.next_serial_line_number - 1),
                        checksum: true,
                    },
                    context,
                );

                self.on_ok = OnOK::IgnoreOK;
            }
            OnOK::IgnoreOK => {
                // Do not cancel the trickle here because it will already have been overwritten by the resend request
                self.on_ok = OnOK::Despool;
            }
            OnOK::TransitionToReady => {
                eprintln!("Connected");
                context.handle_state_change(&Ready( self.clone() ));

                // let the protobuf clients know that the machine is ready
                effects.push(Effect::ProtobufSend);

                self.on_ok = OnOK::Despool;
                self.receive_ok(effects, context);
            }
            OnOK::Despool => {
                if let Some(poll_for) = self.poll_for {
                    self.poll_feedback(effects, context, poll_for);
                } else {
                    self.despool_task(effects, context);
                }
            }
        }
    }

    fn despool_task(&mut self, effects: &mut Vec<Effect>, context: &mut Context) {
        if let Some(task) = self.task.as_mut() {
            let gcode = task.gcode_lines.next();

            if let Some(gcode) = gcode {
                context.push_gcode_tx(gcode.clone());
                send_serial(
                    effects,
                    GCodeLine {
                        gcode,
                        line_number: Some(self.next_serial_line_number),
                        checksum: true,
                    },
                    context,
                );

                self.on_ok = OnOK::Despool;
                self.next_serial_line_number += 1;
                context.feedback.despooled_line_number += 1;
            } else {
                // record a task completion event
                context.push_finish_task(&task);

                self.task = None;
                context.feedback.despooled_line_number = 0;
                self.on_ok = OnOK::NotAwaitingOk;

                // Cancel the tickle if there's nothing else to send_serial
                effects.push(
                    Effect::CancelDelay { key: "tickle_delay".to_string() }
                );
                effects.push(
                    Effect::ProtobufSend,
                );

                if context.reset_when_idle {
                    effects.push(Effect::ExitProcessAfterDelay)
                };
            }
        } else {
            self.on_ok = OnOK::NotAwaitingOk;
            effects.push(
                Effect::CancelDelay { key: "tickle_delay".to_string() }
            );
        }
    }

    fn poll_feedback(
        &mut self,
        effects: &mut Vec<Effect>,
        context: &mut Context,
        poll_for: Polling,
    ) {
        let gcode = match poll_for {
            Polling::PollTemperature => "M105",
            Polling::PollPosition => "M114",
        };

        send_serial(
            effects,
            GCodeLine {
                gcode: gcode.to_string(),
                line_number: Some(self.next_serial_line_number),
                checksum: true,
            },
            context,
        );

        self.on_ok = OnOK::Despool;
        self.next_serial_line_number += 1;

        self.poll_for = match poll_for {
            Polling::PollPosition => Some(Polling::PollTemperature),
            Polling::PollTemperature => None,
        };
    }

    fn tickle_serial_port(mut self, context: &mut Context) -> Loop {
        let response_timeout_tickle_attempts = context.controller.response_timeout_tickle_attempts;
        let checksum_tickles = context.controller.checksum_tickles;

        if self.tickles_attempted >= response_timeout_tickle_attempts {
            let message = "Serial port communication timed out.".to_string();
            errored(message, context)
        } else {
            eprintln!("Warning: GCode acknowledgement not received. Attempting to continue.");

            let mut effects = vec![];

            // Send M105 without a line number or a checksum. This should cause the
            // printer to return an "OK" and then the line number will be correct
            // unless a line has been missed (in which case we are in an error state
            // and the print will then be aborted by a firmware line number error as it should be).
            //
            // Should be equivalent to Octoprint's tickle logic:
            // https://github.com/foosel/OctoPrint/blob/master/src/octoprint/util/comm.py#L2302
            send_serial(
                &mut effects,
                GCodeLine {
                    // gcode: format!("M110 N{:}", self.next_serial_line_number - 1),
                    gcode: format!("M105"),
                    line_number: None,
                    checksum: checksum_tickles,
                },
                context,
            );

            self.tickles_attempted += 1;

            Loop::new(Ready(self), effects)
        }
    }

    fn receive_resend_request(mut self, line_number: u32, context: &mut Context) -> Loop {
        let sent_line_number = self.next_serial_line_number - 1;

        /*
        * Teg only sends one line at a time. If a resend is requested for a
        * different line number then this is likely an issue of the printer's
        * firmware.
        */
        if line_number != sent_line_number {
            let message = format!(
                "resend line number {:?} does not match sent line number {:?}",
                line_number,
                sent_line_number,
            );

            errored(message, context)
        } else {
            // wait for the ok sent after the resend (see marlinFixture.js)
            self.on_ok = OnOK::Resend;

            Ready(self).and_no_effects()
        }
    }
}
