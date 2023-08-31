// Special wrappers for the endpoints for HF

use axum::{extract::Path, http::StatusCode, Json};
use hf_hub::{api::tokio::Api, Repo};
use time::OffsetDateTime;

use crate::api_types::{HFFile, ListHFFilesResponse};

#[axum::debug_handler]
pub async fn ls_repo_files(
    Path((community, repo_name)): Path<(String, String)>,
) -> Result<Json<ListHFFilesResponse>, StatusCode> {
    // Write some text to a file
    std::fs::write("/tmp/log", "import 1\n").unwrap();
    let hf = Api::new().unwrap();
    std::fs::write("/tmp/log", "import 2\n").unwrap();
    let repo = hf.repo(Repo::new(
        format!("{}/{}", &community, &repo_name),
        hf_hub::RepoType::Model,
    ));
    std::fs::write("/tmp/log", "import 3\n").unwrap();
    let files = repo
        .info()
        .await
        .expect("failed to get repo files")
        .siblings;
    std::fs::write("/tmp/log", "import 4\n").unwrap();
    // For every file we include it here.
    let hf_files = files
        .iter()
        .map(|f| HFFile {
            committed_at: OffsetDateTime::now_utc(),
            filename: f.rfilename.clone(),
            size_bytes: 0,
            subfolder: None,
        })
        .collect();
    std::fs::write("/tmp/log", "import 5\n").unwrap();

    Ok(Json(ListHFFilesResponse {
        repo: repo_name,
        files: hf_files,
    }))
}
