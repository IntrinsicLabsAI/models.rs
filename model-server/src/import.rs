use std::{collections::HashMap, path::PathBuf, sync::Arc};

use anyhow::{Context, Ok};
use axum::async_trait;
use log::info;
use tokio::sync::{
    mpsc::{self, Sender},
    RwLock,
};

use crate::router::models::types::{DiskLocator, HFLocator};

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
    /// - **[Completed]** - for imports that are complete and cached locally on disk
    /// - **[Failed]** - for import jobs that failed with an error
    #[derive(Debug, Clone, Serialize)]
    #[serde(tag = "type")]
    pub enum ImportJobStatus {
        #[serde(rename = "queued")]
        Queued,

        #[serde(rename = "in-progress")]
        InProgress {
            /// A numberic progress indicator between 0 (0%) and 1.0 (100%)
            progress: f32,
        },

        #[serde(rename = "completed")]
        Completed { info: Option<String> },

        #[serde(rename = "finished")]
        Failed {
            // We need to keep track of an error, so that it's sendable, and so that we can log it for later.
            error: Option<String>,
        },
    }
}

/// Importer is the trait for types that can conduct external imports.
/// They receive an [ImportTask] which describes the source of the import along with
/// any associated metadata necessary to execute the import.
#[async_trait]
pub trait Importer {
    async fn start_import(&self, task: ImportJob) -> anyhow::Result<ImportJobId>;
    async fn get_import_status(&self, task_id: &ImportJobId) -> anyhow::Result<ImportJobStatus>;
    async fn get_all_job_status(&self) -> anyhow::Result<HashMap<ImportJobId, ImportJobStatus>>;
}

#[derive(Debug)]
struct JobEntry {
    task: ImportJob,
    status: ImportJobStatus,
}

/// Message used by our async task queue which interposes between the main task and the worker tasks doing
/// the downloading.
enum Message {
    UpdateStatus {
        job: ImportJobId,
        status: ImportJobStatus,
    },
}

/// The default in-memory importer implementation. Uses a multi-producer single-consumer
/// task structure to asynchronously download models and update the state tracker.
pub struct InMemoryImporter {
    /// Synchronized table of job statuses. This typestring is gross AF
    job_status: Arc<RwLock<HashMap<ImportJobId, JobEntry>>>,

    /// mpsc message channel for communication between the workers and the state-tracker.
    sender: Sender<Message>,

    /// Root location of where models are extracted to disk.
    root_dir: PathBuf,
}

impl InMemoryImporter {
    pub fn new() -> Self {
        // TODO(aduffy): should this be bounded? Or what should the bound be if not?
        let (sender, mut receiver) = mpsc::channel::<Message>(128);
        let job_status = Arc::new(RwLock::new(HashMap::<ImportJobId, JobEntry>::new()));

        let table_clone = Arc::clone(&job_status);
        tokio::spawn(async move {
            info!("Spawning background task for DefaultImporter");
            while let Some(msg) = receiver.recv().await {
                match msg {
                    Message::UpdateStatus { job, status } => {
                        info!("Updating task={} status={:?}", job, &status);
                        let mut table = table_clone.write().await;
                        if let Some(entry) = table.get_mut(&job) {
                            entry.status = status;
                        }
                    }
                }
            }
            info!("Completing");
        });

        Self {
            job_status,
            sender,
            root_dir: PathBuf::from("value"),
        }
    }
}

async fn do_import(
    task_id: ImportJobId,
    task: ImportJob,
    sender: mpsc::Sender<Message>,
) -> anyhow::Result<()> {
    info!("Job status updating: {:?}", &task);

    // Get the progress updaters here...
    match &task {
        ImportJob::DISK { locator } => import_disk(locator).await,
        ImportJob::HF { locator } => import_hf(locator).await,
    };

    sender
        .send(Message::UpdateStatus {
            job: task_id.clone(),
            status: ImportJobStatus::InProgress { progress: 0.0 },
        })
        .await
        .context("failed to send status update")?;

    Ok(())
}

async fn import_hf(_locator: &HFLocator) {}

async fn import_disk(_locator: &DiskLocator) {}

#[async_trait]
impl Importer for InMemoryImporter {
    async fn start_import(&self, task: ImportJob) -> anyhow::Result<ImportJobId> {
        let result = match &task {
            ImportJob::HF { locator: _ } => {
                return Err(anyhow::Error::msg("hf imports not supported yet!"));
            }

            ImportJob::DISK { locator: _ } => {
                let task_id = uuid::Uuid::new_v4();

                {
                    let mut jq = self.job_status.write().await;
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

    async fn get_import_status(&self, task_id: &ImportJobId) -> anyhow::Result<ImportJobStatus> {
        // Print out the status of the first job
        let jq = self.job_status.read().await;
        if let Some(value) = jq.get(task_id) {
            return Ok(value.status.clone());
        }

        Err(anyhow::Error::msg("oopsie, no data"))
    }

    async fn get_all_job_status(&self) -> anyhow::Result<HashMap<ImportJobId, ImportJobStatus>> {
        let _ = self.job_status.read().await;
        let mut hm = HashMap::new();
        for (k, v) in self.job_status.read().await.iter() {
            hm.insert(k.clone(), v.status.clone());
        }

        Ok(hm)
    }
}
