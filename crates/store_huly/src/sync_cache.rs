use crate::api::Timestamp;
use reqwest::Client;
use rustical_store::Error;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::account_api::{TokenClaims, generate_token};
use crate::auth::HulyUser;

#[async_trait::async_trait]
pub trait SyncCache: Send + Sync + std::fmt::Debug {
    async fn get_sync_state(
        &self,
        user: &HulyUser,
        synctoken: i64,
    ) -> Result<Vec<(String, Timestamp)>, Error>;
    async fn set_sync_state(
        &self,
        user: &HulyUser,
        synctoken: i64,
        events: &Vec<(String, Timestamp)>,
    ) -> Result<(), Error>;
    async fn get_latest_synctoken(&self, user: &HulyUser) -> Result<i64, Error>;
}

/// Calculate a hash for a collection of events with their modification timestamps
pub fn calculate_hash(events: &[(String, Timestamp)]) -> u64 {
    let mut hasher = DefaultHasher::new();
    events.hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug)]
pub struct HttpSyncCache {
    client: Client,
    base_url: String,
    namespace: String,
    secret: String,
}

impl HttpSyncCache {
    pub fn new(kv_url: String, secret: String) -> Self {
        Self {
            client: Client::new(),
            base_url: kv_url.into(),
            namespace: "caldav_synctoken".into(),
            secret,
        }
    }

    fn get_key(&self, user: &HulyUser, synctoken: i64) -> String {
        format!(
            "{}_{}_{}",
            user.id,
            user.workspace_url.replace("/", "_"),
            synctoken
        )
    }

    fn generate_auth_token(&self) -> Result<String, Error> {
        // Create token with only the "extra" claim as per requirements
        let claims = TokenClaims {
            account: "",
            workspace: None,
            extra: Some(HashMap::from([("service", "caldav")])),
        };

        let token = generate_token(&claims, &self.secret)?;
        Ok(format!("Bearer {}", token))
    }
}

#[async_trait::async_trait]
impl SyncCache for HttpSyncCache {
    async fn get_sync_state(
        &self,
        user: &HulyUser,
        synctoken: i64,
    ) -> Result<Vec<(String, Timestamp)>, Error> {
        let key = self.get_key(user, synctoken);
        let url = format!("{}/api/{}/{}", self.base_url, self.namespace, key);

        let auth_token = self.generate_auth_token()?;
        let response = self
            .client
            .get(&url)
            .header("Authorization", auth_token)
            .send()
            .await
            .map_err(|e| Error::ApiError(format!("Failed to fetch sync state: {}", e)))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(Vec::new());
        }

        let events: Vec<(String, Timestamp)> = response
            .json()
            .await
            .map_err(|e| Error::ApiError(format!("Failed to parse sync state response: {}", e)))?;

        Ok(events)
    }

    async fn set_sync_state(
        &self,
        user: &HulyUser,
        synctoken: i64,
        events: &Vec<(String, Timestamp)>,
    ) -> Result<(), Error> {
        let key = self.get_key(user, synctoken);
        let url = format!("{}/api/{}/{}", self.base_url, self.namespace, key);

        let auth_token = self.generate_auth_token()?;

        self.client
            .post(&url)
            .header("Authorization", auth_token)
            .json(events)
            .send()
            .await
            .map_err(|e| Error::ApiError(format!("Failed to store sync state: {}", e)))?;

        Ok(())
    }

    async fn get_latest_synctoken(&self, user: &HulyUser) -> Result<i64, Error> {
        // Create a prefix for the user's keys
        let prefix = format!("{}_{}_", user.id, user.workspace_url.replace("/", "_"));
        let url = format!("{}/api/{}?prefix={}", self.base_url, self.namespace, prefix);

        // Get all keys with the prefix
        let auth_token = self.generate_auth_token()?;
        let response = self
            .client
            .get(&url)
            .header("Authorization", auth_token)
            .send()
            .await
            .map_err(|e| Error::ApiError(format!("Failed to fetch synctoken keys: {}", e)))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(0);
        }

        #[derive(Deserialize)]
        struct KeysResponse {
            keys: Vec<String>,
        }

        let keys_response: KeysResponse = response
            .json()
            .await
            .map_err(|e| Error::ApiError(format!("Failed to parse keys response: {}", e)))?;

        let mut synctoken = 0;
        for key in keys_response.keys {
            // Extract the synctoken part from the key
            let parts: Vec<&str> = key.split('_').collect();
            if parts.len() >= 3 {
                if let Ok(token) = parts[2].parse::<i64>() {
                    synctoken = synctoken.max(token);
                }
            }
        }
        Ok(synctoken)
    }
}

/// A file-based implementation of the SyncCache trait
#[derive(Debug)]
pub struct FileSyncCache {
    base_path: PathBuf,
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

    fn get_file_path(&self, user: &HulyUser, synctoken: i64) -> PathBuf {
        let user_dir = self.base_path.join(format!(
            "{}_{}",
            user.id,
            user.workspace_url.replace("/", "_")
        ));

        // Ensure the user directory exists
        if !user_dir.exists() {
            fs::create_dir_all(&user_dir).expect("Failed to create user sync cache directory");
        }

        user_dir.join(format!("sync_{}.json", synctoken))
    }
}

#[async_trait::async_trait]
impl SyncCache for FileSyncCache {
    async fn get_sync_state(
        &self,
        user: &HulyUser,
        synctoken: i64,
    ) -> Result<Vec<(String, Timestamp)>, Error> {
        let file_path = self.get_file_path(user, synctoken);

        if !file_path.exists() {
            return Ok(Vec::new());
        }

        let mut file = fs::File::open(&file_path)
            .map_err(|e| Error::ApiError(format!("Failed to open sync cache file: {}", e)))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| Error::ApiError(format!("Failed to read sync cache file: {}", e)))?;

        let events: Vec<(String, Timestamp)> = serde_json::from_str(&contents)
            .map_err(|e| Error::ApiError(format!("Failed to parse sync cache file: {}", e)))?;

        Ok(events)
    }

    async fn set_sync_state(
        &self,
        user: &HulyUser,
        synctoken: i64,
        events: &Vec<(String, Timestamp)>,
    ) -> Result<(), Error> {
        let file_path = self.get_file_path(user, synctoken);

        let json = serde_json::to_string_pretty(events)
            .map_err(|e| Error::ApiError(format!("Failed to serialize sync state: {}", e)))?;

        let mut file = fs::File::create(&file_path)
            .map_err(|e| Error::ApiError(format!("Failed to create sync cache file: {}", e)))?;

        file.write_all(json.as_bytes())
            .map_err(|e| Error::ApiError(format!("Failed to write sync cache file: {}", e)))?;

        Ok(())
    }

    async fn get_latest_synctoken(&self, user: &HulyUser) -> Result<i64, Error> {
        let user_dir = self.base_path.join(format!(
            "{}_{}",
            user.id,
            user.workspace_url.replace("/", "_")
        ));

        if !user_dir.exists() {
            return Ok(1); // Start with 1 if no synctokens exist
        }

        // Read all files in the user directory
        let entries = fs::read_dir(&user_dir)
            .map_err(|e| Error::ApiError(format!("Failed to read sync cache directory: {}", e)))?;

        // Extract synctoken values from filenames
        let mut synctokens: Vec<i64> = Vec::new();
        for entry in entries {
            if let Ok(entry) = entry {
                let filename = entry.file_name();
                let filename_str = filename.to_string_lossy();

                // Extract synctoken from filename (format: sync_<token>.json)
                if let Some(token_str) = filename_str
                    .strip_prefix("sync_")
                    .and_then(|s| s.strip_suffix(".json"))
                {
                    if let Ok(token) = token_str.parse::<i64>() {
                        synctokens.push(token);
                    }
                }
            }
        }

        // Sort synctokens to find the highest
        synctokens.sort_unstable();

        // Return the highest synctoken + 1, or 1 if no synctokens exist
        Ok(synctokens.last().map_or(1, |&token| token + 1))
    }
}
