use std::sync::Arc;

use llamacpp::Model;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct LockedModel {
    pub model: Arc<Mutex<Model>>,
}

impl LockedModel {
    pub fn new(model: Model) -> Self {
        LockedModel {
            model: Arc::new(Mutex::new(model)),
        }
    }
}

unsafe impl Send for LockedModel {}
unsafe impl Sync for LockedModel {}
