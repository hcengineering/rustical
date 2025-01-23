pub mod addressbook;
pub mod addressbook_store;
pub mod calendar_store;
pub mod error;
pub use error::Error;
pub mod auth;
pub mod calendar;
//mod contact_birthday_store;
mod subscription_store;
pub mod synctoken;

pub use addressbook_store::AddressbookStore;
pub use calendar_store::CalendarStore;
//pub use contact_birthday_store::ContactBirthdayStore;
pub struct ContactBirthdayStore {}
pub use subscription_store::*;

pub use addressbook::{AddressObject, Addressbook};
pub use calendar::{Calendar, CalendarObject};

#[derive(Debug, Clone)]
pub enum CollectionOperationType {
    // Sync-Token increased
    Object,
    Delete,
}

#[derive(Debug, Clone)]
pub enum CollectionOperationDomain {
    Calendar,
    Addressbook,
}

#[derive(Debug, Clone)]
pub struct CollectionOperation {
    pub r#type: CollectionOperationType,
    pub domain: CollectionOperationDomain,
    pub topic: String,
    pub sync_token: Option<String>,
}
