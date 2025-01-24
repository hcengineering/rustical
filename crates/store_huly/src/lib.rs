mod addressbook_store;
mod auth;
mod calendar_store;
mod subscription_store;

#[derive(Debug)]
pub struct HulyStore {
}

impl HulyStore {
    pub fn new() -> Self {
        Self {}
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
