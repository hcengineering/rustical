use rustical_store::Error;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// A trait for caching sync state information
#[async_trait::async_trait]
pub trait SyncCache: Send + Sync + std::fmt::Debug {
    /// Get the sync state for a given synctoken
    async fn get_sync_state(&self, synctoken: u64) -> Result<Vec<String>, Error>;

    /// Set the sync state for a given synctoken
    async fn set_sync_state(&self, synctoken: u64, event_ids: Vec<String>) -> Result<(), Error>;
}

/// A file-based implementation of the SyncCache trait
#[derive(Debug)]
pub struct FileSyncCache {
    base_path: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct SyncState {
    event_ids: Vec<String>,
}

impl FileSyncCache {
    /// Create a new FileSyncCache with the given base path
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        let path = base_path.as_ref().to_path_buf();
        // Ensure the directory exists
        if !path.exists() {
            fs::create_dir_all(&path).expect("Failed to create sync cache directory");
        }
        Self { base_path: path }
    }

    fn get_file_path(&self, synctoken: u64) -> PathBuf {
        self.base_path.join(format!("sync_{}.json", synctoken))
    }
}

#[async_trait::async_trait]
impl SyncCache for FileSyncCache {
    async fn get_sync_state(&self, synctoken: u64) -> Result<Vec<String>, Error> {
        let file_path = self.get_file_path(synctoken);

        if !file_path.exists() {
            return Ok(Vec::new());
        }

        let mut file = fs::File::open(&file_path)
            .map_err(|e| Error::ApiError(format!("Failed to open sync cache file: {}", e)))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| Error::ApiError(format!("Failed to read sync cache file: {}", e)))?;

        let state: SyncState = serde_json::from_str(&contents)
            .map_err(|e| Error::ApiError(format!("Failed to parse sync cache file: {}", e)))?;

        Ok(state.event_ids)
    }

    async fn set_sync_state(&self, synctoken: u64, event_ids: Vec<String>) -> Result<(), Error> {
        let file_path = self.get_file_path(synctoken);

        let state = SyncState { event_ids };
        let json = serde_json::to_string_pretty(&state)
            .map_err(|e| Error::ApiError(format!("Failed to serialize sync state: {}", e)))?;

        let mut file = fs::File::create(&file_path)
            .map_err(|e| Error::ApiError(format!("Failed to create sync cache file: {}", e)))?;

        file.write_all(json.as_bytes())
            .map_err(|e| Error::ApiError(format!("Failed to write sync cache file: {}", e)))?;

        Ok(())
    }
}
