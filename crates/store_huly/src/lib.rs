pub use calendar_cache::HulyCalendarCache;
use std::sync::Arc;
use tokio::sync::Mutex;

mod addressbook_store;
mod api;
mod auth;
mod calendar_cache;
mod calendar_store;
mod convert;
mod convert_rrule;
mod convert_time;
mod subscription_store;

#[derive(Debug)]
pub struct HulyStore {
    pub(crate) calendar_cache: Arc<Mutex<HulyCalendarCache>>,
}

impl HulyStore {
    pub fn new(calendar_cache: Arc<Mutex<HulyCalendarCache>>) -> Self {
        Self { calendar_cache }
    }
}

#[derive(Debug)]
pub struct HulyAuthProvider {
    accounts_url: String,
    token_expiration: std::time::Duration,
    calendar_cache: Arc<Mutex<HulyCalendarCache>>,
}

impl HulyAuthProvider {
    pub fn new(
        accounts_url: String,
        token_expiration: std::time::Duration,
        calendar_cache: Arc<Mutex<HulyCalendarCache>>,
    ) -> Self {
        Self {
            accounts_url,
            token_expiration,
            calendar_cache,
        }
    }
}
