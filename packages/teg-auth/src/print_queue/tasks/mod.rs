// use std::collections::HashMap;
// use chrono::prelude::*;
use async_graphql::GQLMergedObject;
// use serde::{Deserialize, Serialize};

// use crate::{
//     Query,
//     Mutation
// };

mod models;
pub use models::*;

// Resolvers
mod package_resolvers;
mod part_resolvers;
mod print_queue_resolvers;
pub mod query_resolvers;
mod task_resolvers;

#[path = "mutations/create_job.mutation.rs"]
pub mod create_job_mutation;
use create_job_mutation::CreateJobMutation;

#[path = "mutations/delete_job.mutation.rs"]
pub mod delete_job_mutation;
use delete_job_mutation::DeleteJobMutation;

#[path = "mutations/exec_gcodes.mutation.rs"]
pub mod exec_gcodes_mutation;
use exec_gcodes_mutation::ExecGCodesMutation;

#[path = "mutations/pause_print_mutation.rs"]
pub mod pause_print_mutation;
use pause_print_mutation::PausePrintMutation;

#[path = "mutations/resume_print_mutation.rs"]
pub mod resume_print_mutation;
use resume_print_mutation::ResumePrintMutation;

#[path = "mutations/set_job_position.mutation.rs"]
pub mod set_job_position_mutation;
use set_job_position_mutation::SetJobPositionMutation;

#[path = "mutations/spool_job_file.mutation.rs"]
pub mod spool_job_file_mutation;
use spool_job_file_mutation::SpoolJobFileMutation;

#[derive(GQLMergedObject, Default)]
pub struct PrintQueueMutation(
    CreateJobMutation,
    DeleteJobMutation,
    ExecGCodesMutation,
    PausePrintMutation,
    ResumePrintMutation,
    SetJobPositionMutation,
    SpoolJobFileMutation,
);
