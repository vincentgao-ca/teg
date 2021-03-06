// use async_std::prelude::*;
use futures::{
    TryStreamExt,
    stream::{StreamExt},
    Future,
    Stream,
};
use async_std::os::unix::net::UnixStream;
use futures::future::FutureExt;

use std::sync::Arc;
use anyhow::{
    // anyhow,
    Result,
    // Context as _,
};
// use bytes::BufMut;

use crate::models::versioned_model::{
    VersionedModel,
    Change,
    // Event,
};
use crate::print_queue::tasks::{
    Task,
    TaskStatus,
    // TaskContent,
};
use crate::machine::models::{
    Machine,
    // MachineStatus,
    // Printing,
};

use super::{
    super::{
        spool_task,
        stop_and_reset:: {
            stop_machine,
            reset_machine,
        },
        delete_task_history,
        pause_task,
    },
    send_message,
};

#[derive(PartialEq)]
enum LoopState {
    Continue,
    Done,
}

use futures::channel::mpsc;

fn prefilter<S, F>(
    stream: S,
    f: F,
) -> (impl Future<Output = Result<()>>, mpsc::Receiver<S::Item>)
where
    S: Stream + Send + Unpin,
    S::Item: Send + Unpin,
    F: Send + Unpin + FnMut(&S::Item) -> bool,
{
    // use futures::sink::SinkExt;

    let (tx, rx) = mpsc::channel(1024);
    let runner = prefilter_runner(stream, f, tx);

    (runner, rx)
}

async fn prefilter_runner<S, F>(
    mut stream: S,
    mut f: F,
    mut tx: mpsc::Sender<S::Item>
) -> Result<()>
where
    S: Stream + Send + Unpin,
    S::Item: Send + Unpin,
    F: Send + Unpin + FnMut(&S::Item) -> bool,
{
    use futures::sink::SinkExt;

    while let Some(item) = stream.next().await {
        let passed_filter = f(&item);
        if passed_filter {
            tx.send(item).await?;
        };
    }

    Ok(())
}

pub async fn run_send_loop(
    client_id: u32,
    ctx: Arc<crate::Context>,
    machine_id: u64,
    stream: UnixStream,
) -> Result<()> {
    info!("Machine #{:?}: Send Loop Started", machine_id);

    let ctx_clone = Arc::clone(&ctx);
    let stream_clone = stream.clone();
    let machine_loop = Machine::watch_id_changes(&ctx.db, machine_id)?
        .and_then(move |event| {
            let ctx = Arc::clone(&ctx_clone);
            let stream = stream_clone.clone();

            async move {
                let ctx = Arc::clone(&ctx);
                let mut stream = stream.clone();

                match event {
                    // Machine Stops and Resets
                    Change {
                        next: Some(next_machine),
                        previous: Some(machine),
                        ..
                    } => {
                        info!("Machine Insert");
                        // Stop (from GraphQL mutation)
                        if next_machine.stop_counter != machine.stop_counter {
                            send_message(&mut stream, stop_machine()).await?;
                        }
                        // Reset (from GraphQL mutation)
                        if next_machine.reset_counter != machine.reset_counter {
                            send_message(&mut stream, reset_machine()).await?;
                        }
                        // Paused (from GraphQL mutation)
                        if let Some(task_id) = next_machine.pausing_task_id {
                            if machine.pausing_task_id.is_none() {
                                send_message(&mut stream, pause_task(task_id)).await?;
                            }
                        }
                        // Update task statuses on changes in machine state (eg. machine stops, errors)
                        if
                            machine.status.is_driver_ready()
                            && !next_machine.status.is_driver_ready()
                        {
                            async_std::task::spawn_blocking(move || {
                                for task in Task::scan(&ctx.db) {
                                    let task = task?;

                                    if task.machine_id == machine.id {
                                        Task::get_and_update(
                                            &ctx.db,
                                            task.id,
                                            |mut task| {
                                                if task.status.is_pending() {
                                                    task.status = TaskStatus::Errored;
                                                }
                                                task
                                            }
                                        )?;
                                    }
                                };
                                Result::<()>::Ok(())
                            }).await?;
                        }
                    },
                    // Exit gracefully upon deletion of the machine
                    Change { next: None, .. } => {
                        info!("Machine Deleted");
                        return Ok(LoopState::Done)
                    },
                    _ => {}
                }
                Ok(LoopState::Continue)
            }
        })
        .try_filter_map(|loop_state| {
            let opt = if loop_state == LoopState::Done {
                Some(())
            } else {
                None
            };
            futures::future::ready(Ok(opt))
        })
        .boxed();

    let ctx_clone = Arc::clone(&ctx);
    let stream_clone = stream.clone();
    let task_loop = Task::watch_all_changes(&ctx.db)?;
    let (filterer, task_loop) = prefilter(
        task_loop,
        |change| {
            match change {
                // Task inserts
                Ok(Change { next: Some(task), .. }) => {
                    task.machine_id == machine_id
                    && !task.sent_to_machine
                    && task.status == TaskStatus::Spooled
                }
                // Task deletions
                Ok(Change { previous: Some(task), next: None, .. }) => {
                    task.machine_id == machine_id
                }
                Err(_) => true,
                _ => false,
            }
        },
    );

    let task_loop = task_loop
        .and_then(move |event| {
            let ctx = Arc::clone(&ctx_clone);
            let stream = stream_clone.clone();
            // let machine_id = machine_id.clone();

            async move {
                let ctx = Arc::clone(&ctx);
                let mut stream = stream.clone();
                // let machine_id = machine_id.clone();

                match event {
                    // New and resumed tasks
                    Change {
                        previous,
                        next: Some(task),
                        ..
                    } if
                        previous.as_ref()
                            .map(|t| t.sent_to_machine)
                            .unwrap_or(true)
                        && !task.sent_to_machine
                    => {
                        // Spool new tasks to the driver
                        info!("Sending task to machine");

                        let task = async_std::task::spawn_blocking(move || {
                            Task::get_and_update(
                                &ctx.db,
                                task.id,
                                |mut task| {
                                    task.sent_to_machine = true;
                                    task
                                },
                            )
                        }).await?;

                        send_message(
                            &mut stream,
                            spool_task(client_id, &task)?,
                        ).await?;
                    }
                    // Task deletions
                    Change { previous: Some(task), next: None, .. } => {
                        info!("Task Deleted");
                        // Delete the task from the driver
                        send_message(
                            &mut stream,
                            delete_task_history(task.id),
                        ).await?;
                    }
                    _ => {}
                }
                Ok(())
            }
        })
        .filter(|res| {
            futures::future::ready(res.is_err())
        })
        .boxed();

    // let res = futures::select! {
    //     res = machine_loop.next().fuse() => res.transpose().map(|_| ()),
    //     res = task_loop.next().fuse() => res.transpose().map(|_| ()),
    // };

    let machine_loop = machine_loop.into_future();
    let task_loop = task_loop.into_future();

    let res = futures::select! {
        res = filterer.fuse() => res,
        (res, _) = machine_loop.fuse() => res.transpose().map(|_| ()),
        (res, _) = task_loop.fuse() => res.transpose().map(|_| ()),
    };

    res
}
