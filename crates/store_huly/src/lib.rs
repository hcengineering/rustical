mod addressbook_store;
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
