pub use calendar_cache::HulyCalendarCache;

mod api;
mod addressbook_store;
mod auth;
mod calendar_cache;
mod calendar_store;
mod convert;
mod convert_rrule;
mod convert_time;
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
    api_url: String,
    accounts_url: String,
}

impl HulyAuthProvider {
    pub fn new(api_url: String, accounts_url: String) -> Self {
        Self {
            api_url,
            accounts_url,
        }
    }
}
