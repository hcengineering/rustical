use rustical_frontend::FrontendConfig;
use rustical_oidc::OidcConfig;
use rustical_store::auth::TomlUserStoreConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct HttpConfig {
    pub host: String,
    pub port: u16,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_owned(),
            port: 4000,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SqliteDataStoreConfig {
    pub db_url: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum DataStoreConfig {
    Sqlite(SqliteDataStoreConfig),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum AuthConfig {
    Toml(TomlUserStoreConfig),
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct TracingConfig {
    pub opentelemetry: bool,
    pub log_level: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct DavPushConfig {
    pub enabled: bool,
    #[serde(default)]
    // Allowed Push servers, accepts any by default
    // Specify as URL origins
    pub allowed_push_servers: Option<Vec<String>>,
}

impl Default for DavPushConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_push_servers: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct NextcloudLoginConfig {
    pub enabled: bool,
}

impl Default for NextcloudLoginConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct HulyConfig {
    pub accounts_url: String,
    pub token_expiration_secs: u64,
    pub system_account_uuid: String,
    pub server_secret: String,
    /// When updating, a client makes several calls in sequence
    /// This is not practical to send requests to Huly API at each client's call,
    /// because all of them address the same data.
    /// So after the first call we cache the data for a short period of time
    /// to make subsequent calls faster
    pub cache_invalidation_interval_secs: u64,
    /// Path to store sync cache files
    pub sync_cache_path: Option<String>,
    /// URL to the key-value HTTP service for sync cache
    /// If provided, this will be used instead of file-based sync cache
    pub kv_url: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub data_store: DataStoreConfig,
    pub auth: AuthConfig,
    #[serde(default)]
    pub http: HttpConfig,
    pub frontend: FrontendConfig,
    #[serde(default)]
    pub oidc: Option<OidcConfig>,
    #[serde(default)]
    pub tracing: TracingConfig,
    #[serde(default)]
    pub dav_push: DavPushConfig,
    #[serde(default)]
    pub nextcloud_login: NextcloudLoginConfig,
    pub huly: HulyConfig,
}
