pub use calendar_cache::HulyCalendarCache;

mod api;
mod addressbook_store;
mod auth;
mod calendar_cache;
mod calendar_store;
mod convert;
mod subscription_store;

#[derive(Debug)]
pub struct HulyStore {
    pub(crate) calendar_cache: tokio::sync::Mutex<HulyCalendarCache>,
}

impl HulyStore {
    pub fn new(calendar_cache: tokio::sync::Mutex<HulyCalendarCache>) -> Self {
        Self {
            calendar_cache,
        }
    }
}

pub struct HulyAuthProvider {
    accounts_url: String,
}

impl HulyAuthProvider {
    pub fn new(accounts_url: &str) -> Self {
        Self {
            accounts_url: accounts_url.into(),
        }
    }
}
