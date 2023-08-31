use std::collections::HashMap;

use anyhow::{Context, Ok};
use axum::async_trait;
use log::info;
use tokio::sync::{
    mpsc::{self, Sender},
    RwLock,
};

use self::types::{ImportJob, ImportJobId, ImportJobStatus};

pub mod types {
    use serde::{Deserialize, Serialize};

    use crate::router::models::types::{DiskLocator, HFLocator};

    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
    pub enum ImportJob {
        // Depending on the task, we want to include the subtypes of the locator here as well instead...fuck
        HF { locator: HFLocator },
        DISK { locator: DiskLocator },
    }

    // Have it enqueue a task, and return an ID
    pub type ImportJobId = uuid::Uuid;

    /// Status of an import job.
    /// Import jobs can be in one of three different states at a given point in time
    /// - **[Queued]** - for imports that are taking too long
    /// - **[InProgress]** - for imports that are actively being worked on
    #[derive(Debug)]
    pub enum ImportJobStatus {
        Queued,
        InProgress {
            /// A numberic progress indicator between 0 (0%) and 1.0 (100%)
            progress: f32,
        },
        Completed {
            info: Option<String>,
        },
        Failed {
            error: Box<anyhow::Error>,
        },
    }
}

// The importer will update the status and notify any listeners about the new status.

/// Importer is the trait for types that can conduct external imports.
/// They receive an [ImportTask] which describes the source of the import along with
/// any associated metadata necessary to execute the import.
#[async_trait]
pub trait Importer {
    async fn start_import(&mut self, task: ImportJob) -> anyhow::Result<ImportJobId>;
    async fn get_import_status(&self, task_id: &ImportJobId) -> anyhow::Result<ImportJobStatus>;
}

struct JobEntry {
    task: ImportJob,
    status: ImportJobStatus,
}

pub struct DefaultImporter {
    job_table: RwLock<HashMap<ImportJobId, JobEntry>>,
    sender: Sender<Message>,
}

/// Message used by our async task queue which interposes between the main task and the worker tasks doing
/// the downloading.
enum Message {
    UpdateStatus {
        job: ImportJobId,
        status: ImportJobStatus,
    },
}

impl DefaultImporter {
    pub fn new() -> Self {
        // TODO(aduffy): should this be bounded? Or what should the bound be if not?
        let (sender, mut receiver) = mpsc::channel::<Message>(128);
        let job_table: RwLock<HashMap<ImportJobId, JobEntry>> = RwLock::new(HashMap::new());

        // This background task is responsible for executing.
        tokio::spawn(async move {
            info!("Spawning background task for DefaultImporter");
            while let Some(msg) = receiver.recv().await {
                match msg {
                    Message::UpdateStatus { job, status } => {
                        // Perform update locally
                        let mut table = job_table.write().await;
                        if let Some(entry) = table.get_mut(&job) {
                            entry.status = status;
                        }
                    }
                }
            }
            info!("Completing");
        });

        Self {
            job_table: RwLock::new(HashMap::new()),
            sender,
        }
    }
}

async fn do_import(
    task_id: ImportJobId,
    task: ImportJob,
    sender: mpsc::Sender<Message>,
) -> anyhow::Result<()> {
    info!("Job status updating: {:?}", &task);

    sender
        .send(Message::UpdateStatus {
            job: task_id.clone(),
            status: ImportJobStatus::InProgress { progress: 0.0 },
        })
        .await
        .context("failed to send status update")?;

    Ok(())
}

#[async_trait]
impl Importer for DefaultImporter {
    async fn start_import(&mut self, task: ImportJob) -> anyhow::Result<ImportJobId> {
        let result = match &task {
            ImportJob::HF { locator: _ } => {
                return Err(anyhow::Error::msg("hf imports not supported yet!"));
            }

            ImportJob::DISK { locator: _ } => {
                let task_id = uuid::Uuid::new_v4();

                {
                    let mut jq = self.job_table.write().await;
                    jq.insert(
                        task_id,
                        JobEntry {
                            task: task.clone(),
                            status: ImportJobStatus::Queued,
                        },
                    );
                }

                // Submit an async task to execute against the data, updating the jobs table as relevant.
                let sender = self.sender.clone();
                tokio::spawn(do_import(task_id, task.clone(), sender));

                Ok(task_id)
            }
        };

        result
    }

    async fn get_import_status(&self, _task_id: &ImportJobId) -> anyhow::Result<ImportJobStatus> {
        todo!()
    }
}
