use crate::{
    api_types::{
        CompletionModelParams, DiskLocator, HFLocator, ImportJob, ImportJobId, ImportJobStatus,
        ImportMetadata, ImportSource, ModelParams, ModelType, RegisterModelRequest, Runtime,
    },
    db::tables::DB,
};
use anyhow::{Context, Ok};
use axum::async_trait;
use hf_hub::{api::tokio::Api, Repo};
use log::info;
use semver::Version;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use time::OffsetDateTime;
use tokio::sync::{
    mpsc::{channel, Sender},
    RwLock,
};

/// Importer is the trait for types that can conduct external imports.
/// They receive an [ImportTask] which describes the source of the import along with
/// any associated metadata necessary to execute the import.
#[async_trait]
pub trait Importer {
    async fn start_import(&self, task: ImportJob) -> anyhow::Result<ImportJobId>;
    async fn get_import_status(&self, task_id: &ImportJobId) -> anyhow::Result<ImportJobStatus>;
    async fn get_all_job_status(&self) -> anyhow::Result<HashMap<ImportJobId, ImportJobStatus>>;
}

/// The default in-memory importer implementation. Uses a multi-producer single-consumer
/// task structure to asynchronously download models and update the state tracker.
pub struct InMemoryImporter {
    /// Synchronized table of job statuses. This typestring is gross AF
    job_status: Arc<RwLock<HashMap<ImportJobId, JobEntry>>>,

    /// mpsc message channel for communication between the workers and the state-tracker.
    sender: Sender<Message>,
}

impl InMemoryImporter {
    pub fn new(db: Arc<DB>) -> Self {
        // TODO(aduffy): should this be bounded? Or what should the bound be if not?
        let (sender, mut receiver) = channel::<Message>(128);
        let job_status = Arc::new(RwLock::new(HashMap::<ImportJobId, JobEntry>::new()));

        let table_clone = Arc::clone(&job_status);
        tokio::spawn(async move {
            info!("spawning background task for DefaultImporter");

            while let Some(msg) = receiver.recv().await {
                match msg {
                    Message::UpdateStatus { job, status } => {
                        info!("updating task={} status={:?}", job, &status);

                        let job_def = {
                            // Hold the lock for a very small amount of time
                            let mut table = table_clone.write().await;
                            let entry = table.get_mut(&job).unwrap();
                            entry.status = status.clone();

                            entry.task.clone()
                        };

                        let file_name = match job_def {
                            ImportJob::DISK { ref locator } => locator
                                .path
                                .file_name()
                                .unwrap()
                                .to_owned()
                                .into_string()
                                .unwrap(),
                            ImportJob::HF { ref locator } => locator
                                .file
                                .file_name()
                                .unwrap()
                                .to_owned()
                                .into_string()
                                .unwrap(),
                        };

                        // If update is completed, we need to insert the new model into the DB
                        match status {
                            ImportJobStatus::Completed { info } => {
                                let version = Version::new(0, 1, 0);
                                info!(
                                    "registering model with db name={} version={}",
                                    &file_name, &version
                                );

                                db.register_model(&RegisterModelRequest {
                                    version,
                                    import_metadata: ImportMetadata {
                                        imported_at: OffsetDateTime::now_utc(),
                                        source: match job_def {
                                            ImportJob::HF { ref locator } => ImportSource::HF {
                                                source: locator.clone(),
                                            },
                                            ImportJob::DISK { ref locator } => ImportSource::DISK {
                                                source: locator.clone(),
                                            },
                                        },
                                    },
                                    model: file_name,
                                    model_type: ModelType::Completion,
                                    runtime: Runtime::Ggml,
                                    internal_params: ModelParams::COMPLETION(
                                        CompletionModelParams {
                                            model_path: PathBuf::from(info.unwrap()),
                                        },
                                    ),
                                })
                                .await
                                .unwrap();
                            }
                            _ => (),
                        }
                    }
                }
            }
        });

        Self { job_status, sender }
    }
}

#[async_trait]
impl Importer for InMemoryImporter {
    async fn start_import(&self, task: ImportJob) -> anyhow::Result<ImportJobId> {
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

    async fn get_import_status(&self, task_id: &ImportJobId) -> anyhow::Result<ImportJobStatus> {
        // Print out the status of the first job
        let jq = self.job_status.read().await;
        if let Some(value) = jq.get(task_id) {
            return Ok(value.status.clone());
        }

        Err(anyhow::anyhow!("oopsie, no data"))
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

async fn do_import(
    task_id: ImportJobId,
    task: ImportJob,
    sender: Sender<Message>,
) -> anyhow::Result<()> {
    info!("Job status updating: {:?}", &task);

    sender
        .send(Message::UpdateStatus {
            job: task_id.clone(),
            status: ImportJobStatus::InProgress { progress: 0.0 },
        })
        .await
        .context("failed to send in-progress update")?;

    let download_path = match &task {
        ImportJob::DISK { locator } => import_disk(locator).await,
        ImportJob::HF { locator } => import_hf(locator).await,
    };

    sender
        .send(Message::UpdateStatus {
            job: task_id.clone(),
            status: ImportJobStatus::Completed {
                info: download_path.to_str().map(|p| p.to_string()),
            },
        })
        .await
        .context("failed to send completion update")
}

async fn import_hf(locator: &HFLocator) -> PathBuf {
    let client = Api::new().unwrap();
    info!("Executing download from HF");
    // Send a stream of results back
    let download = client
        .repo(Repo::model(locator.repo.clone()))
        .get(locator.file.to_str().unwrap())
        .await
        .unwrap();

    info!("Download completed target={:?}", &download);
    download
}

async fn import_disk(locator: &DiskLocator) -> PathBuf {
    info!("Doing nothing here");

    locator.path.clone()
}
