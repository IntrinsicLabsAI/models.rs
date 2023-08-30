use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub(crate) enum Migration {
    V0,
}

impl Migration {
    pub fn execute(&self) {
        match self {        
            Self::V0 => {
                // What to do in this case if possible
            },
        }
    }
}

// Init the migration table against a rusqlite connection