// use async_std::prelude::*;
use chrono::{ prelude::*, Duration };
use machine_message::Feedback;

use std::convert::TryInto;
// use std::sync::Arc;
use anyhow::{
    anyhow,
    Result,
    // Context as _,
};

use teg_protobufs::{
    machine_message::{self, Status},
};

use crate::{machine::MachineData, task::{
    Task,
    // TaskContent,
}};
use crate::machine::{
    Machine,
    models::{
        MachineStatus,
        GCodeHistoryEntry,
        GCodeHistoryDirection,
        Errored,
    },
};
use crate::components::{
    // HeaterEphemeral,
    TemperatureHistoryEntry,
    // Axis,
    // SpeedController,
};

pub async fn record_feedback(machine: &mut Machine, feedback: Feedback) -> Result<()> {
    let db = machine.db.clone();
    let machine_data = machine.get_data()?;
    let transaction = db.begin().await?;

    update_tasks(&db, &feedback).await?;
    update_heaters(machine_data, &feedback).await?;
    update_axes(machine_data, &feedback).await?;
    update_speed_controllers(machine_data, &feedback).await?;

    update_machine(machine_data, &feedback).await?;

    transaction.commit().await?;

    Ok(())
}

pub async fn update_tasks(db: &crate::Db, feedback: &Feedback) -> Result<()> {
    for progress in feedback.task_progress.iter() {
        let status = progress.try_into()?;

        let mut task = Task::get(db, progress.task_id).await?;

        trace!("Task #{} status: {:?}", task.id, status);
        task.despooled_line_number = Some(progress.despooled_line_number as u64);
        task.status = status;

        task.update(db).await?;
    }

    Ok(())
}

pub async fn update_heaters(machine: &mut MachineData, feedback: &Feedback) -> Result<()> {
    for h in feedback.heaters.iter() {
        let heater = machine.config.get_heater_mut(&h.address);

        let heater = if let Some(heater) = heater {
            heater
        } else {
            warn!("Heater not found: {}", h.address);
            continue
        };

        let history = &mut heater.history;

        // record a data point once every half second
        if
            history.back()
                .map(|last|
                    Utc::now() > last.created_at + Duration::milliseconds(500)
                )
                .unwrap_or(true)
        {
            history.push_back(
                TemperatureHistoryEntry {
                    target_temperature: Some(h.target_temperature),
                    actual_temperature: Some(h.actual_temperature),
                    ..TemperatureHistoryEntry::new()
                }
            );
        }

        // limit the history to 60 entries (30 seconds)
        const MAX_HISTORY_LENGTH: usize = 60;
        while history.len() > MAX_HISTORY_LENGTH {
            history.pop_front();
        };

        heater.target_temperature = Some(h.target_temperature);
        heater.actual_temperature = Some(h.actual_temperature);
        heater.enabled = h.enabled;
        heater.blocking = h.blocking;
    }

    Ok(())
}

pub async fn update_axes(machine: &mut MachineData, feedback: &Feedback) -> Result<()> {
    let axes = &mut machine.config.axes;

    for a in feedback.axes.iter() {
        let axis = axes.iter_mut()
            .find(|c| c.model.address == a.address)
            .map(|c| &mut c.ephemeral);

        let axis = if let Some(axis) = axis {
            axis
        } else {
            warn!("axes not found: {}", a.address);
            continue
        };

        axis.target_position = Some(a.target_position);
        axis.actual_position = Some(a.actual_position);
        axis.homed = a.homed;
    }

    Ok(())
}

pub async fn update_speed_controllers(machine: &mut MachineData, feedback: &Feedback) -> Result<()> {
    let speed_controllers = &mut machine.config.speed_controllers;

    for sc in feedback.speed_controllers.iter() {
        let sc_eph = speed_controllers.iter_mut()
            .find(|c| c.model.address == sc.address)
            .map(|c| &mut c.ephemeral);

        let sc_eph = if let Some(sc_eph) = sc_eph {
            sc_eph
        } else {
            warn!("speed_controllers not found: {}", sc.address);
            continue
        };

        sc_eph.target_speed = Some(sc.target_speed);
        sc_eph.actual_speed = Some(sc.actual_speed);
        sc_eph.enabled = sc.enabled;
    }

    Ok(())
}

pub async fn update_machine(machine: &mut MachineData, feedback: &Feedback) -> Result<()> {
    trace!("Feedback status: {:?}", feedback.status);
    // Update machine status
    machine.status = match feedback.status {
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

    machine.motors_enabled = feedback.motors_enabled;

    // Update GCode History
    let history = &mut machine.gcode_history;

    for entry in feedback.gcode_history.iter() {
        let direction = if entry.direction == 0 {
            GCodeHistoryDirection::Tx
        } else {
            GCodeHistoryDirection::Rx
        };

        history.push_back(
            GCodeHistoryEntry::new(
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
