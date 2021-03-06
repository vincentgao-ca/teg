// use async_std::prelude::*;
use chrono::{ prelude::*, Duration };
use xactor::{
    Handler,
    Context as XContext,
    message,
};

use std::convert::TryInto;
// use std::sync::Arc;
use anyhow::{
    anyhow,
    Result,
    // Context as _,
};
// use bytes::BufMut;

use teg_protobufs::{
    // MachineMessage,
    // Message,
    machine_message::{self, Status},
};

use crate::models::VersionedModel;
use crate::print_queue::tasks::{
    Task,
    // TaskContent,
};

use crate::machine::models::{
    Machine,
    MachineStatus,
    Errored,
    Heater,
    TemperatureHistoryEntry,
    Axis,
    SpeedController,
    GCodeHistoryDirection,
    GCodeHistoryEntry,
};

use super::EphemeralMachineData;

#[message(result = "Result<()>")]
pub struct RecordFeedback(pub machine_message::Feedback);

#[async_trait::async_trait]
impl Handler<RecordFeedback> for EphemeralMachineData {
    async fn handle(&mut self, _ctx: &mut XContext<Self>, msg: RecordFeedback) -> Result<()> {
        let feedback = &msg.0;

        // Record task progress
        for progress in feedback.task_progress.iter() {
            let status = progress.try_into()?;

            Task::get_opt_and_update(
                &self.db,
                progress.task_id as u64,
                |task| task.map(|mut task| {
                    trace!("Task #{} status: {:?}", task.id, status);
                    task.despooled_line_number = Some(progress.despooled_line_number as u64);
                    task.status = status;
                    task
                })
            )?;
        }

        trace!("Feedback status: {:?}", feedback.status);
        // Update machine status
        let next_machine_status = match feedback.status {
            i if i == Status::Errored as i32 && feedback.error.is_some() => {
                let message = feedback.error.as_ref().unwrap().message.clone();
                MachineStatus::Errored(Errored { message })
            }
            i if i == Status::Estopped as i32 => MachineStatus::Stopped,
            i if i == Status::Disconnected as i32 => MachineStatus::Disconnected,
            i if i == Status::Connecting as i32 => MachineStatus::Connecting,
            i if i == Status::Ready as i32 => MachineStatus::Ready,
            i => Err(anyhow!("Invalid machine status: {:?}", i))?,
        };

        let motors_enabled = feedback.motors_enabled;

        Machine::set_status(
            &self.db,
            self.id,
            |machine| {
                machine.motors_enabled = motors_enabled;
                next_machine_status.clone()
            },
        )?;

        // Update heaters
        let heaters = Heater::scan(&self.db)
            .collect::<Result<Vec<Heater>>>()?;

        for h in feedback.heaters.iter() {
            let id = heaters.iter()
                .find(|heater| heater.address == h.address)
                .map(|heater| heater.id);

            let id = if let Some(id) = id {
                id
            } else {
                warn!("Heater not found: {}", h.address);
                continue
            };

            let temperature_history_id = Heater::generate_id(&self.db)?;

            Heater::get_and_update(&self.db, id, |mut heater| {
                let history = &mut heater.history;

                // record a data point once every half second
                if
                    history.back()
                        .map(|last| Utc::now() > last.created_at + Duration::milliseconds(500))
                        .unwrap_or(true)
                {
                    history.push_back(
                        TemperatureHistoryEntry {
                            target_temperature: Some(h.target_temperature),
                            actual_temperature: Some(h.actual_temperature),
                            ..TemperatureHistoryEntry::new(temperature_history_id)
                        }
                    );
                }

                // limit the history to 60 entries (30 seconds)
                const MAX_HISTORY_LENGTH: usize = 60;
                while history.len() > MAX_HISTORY_LENGTH {
                    history.pop_front();
                };

                Heater {
                    target_temperature: Some(h.target_temperature),
                    actual_temperature: Some(h.actual_temperature),
                    enabled: h.enabled,
                    blocking: h.blocking,
                    ..heater
                }
            })?;
        }

        // Update axes
        let axes = Axis::scan(&self.db)
            .collect::<Result<Vec<Axis>>>()?;

        for a in feedback.axes.iter() {
            let id = axes.iter()
                .find(|axis| axis.address == a.address)
                .map(|axis| axis.id);

            let id = if let Some(id) = id {
                id
            } else {
                warn!("axes not found: {}", a.address);
                continue
            };

            Axis::get_and_update(&self.db, id, |axis| {
                Axis {
                    target_position: Some(a.target_position),
                    actual_position: Some(a.actual_position),
                    homed: a.homed,
                    ..axis
                }
            })?;
        }

        // Update speed controllers
        let speed_controllers = SpeedController::scan(&self.db)
            .collect::<Result<Vec<SpeedController>>>()?;

        for sc in feedback.speed_controllers.iter() {
            let id = speed_controllers.iter()
                .find(|speed_controller| speed_controller.address == sc.address)
                .map(|speed_controller| speed_controller.id);

            let id = if let Some(id) = id {
                id
            } else {
                warn!("speed_controllers not found: {}", sc.address);
                continue
            };

            SpeedController::get_and_update(&self.db, id, |speed_controller| {
                SpeedController {
                    target_speed: Some(sc.target_speed),
                    actual_speed: Some(sc.actual_speed),
                    enabled: sc.enabled,
                    ..speed_controller
                }
            })?;
        }

        // Update GCode History
        let history = &mut self.gcode_history;

        for entry in feedback.gcode_history.iter() {
            let direction = if entry.direction == 0 {
                GCodeHistoryDirection::Tx
            } else {
                GCodeHistoryDirection::Rx
            };

            history.push_back(
                GCodeHistoryEntry::new(
                    Machine::generate_id(&self.db)?,
                    entry.content.clone(),
                    direction,
                )
            );

            const MAX_HISTORY_LENGTH: usize = 400;
            while history.len() > MAX_HISTORY_LENGTH {
                history.pop_front();
            };
        }

        Ok(())
    }
}
