use async_trait::async_trait;
use rustical_store::{AddressObject, Addressbook, AddressbookStore, Error};
use super::HulyStore;

#[async_trait]
impl AddressbookStore for HulyStore {
    async fn get_addressbook(&self, principal: &str, id: &str) -> Result<Addressbook, Error> {
        Err(Error::NotFound)
    }

    async fn get_addressbooks(&self, principal: &str) -> Result<Vec<Addressbook>, Error> {
        Err(Error::NotFound)
    }

    async fn get_deleted_addressbooks(&self, principal: &str) -> Result<Vec<Addressbook>, Error> {
        Err(Error::NotFound)
    }

    async fn update_addressbook(
        &self,
        principal: String,
        id: String,
        addressbook: Addressbook,
    ) -> Result<(), Error> {
        Err(Error::NotFound)
    }

    async fn insert_addressbook(&self, addressbook: Addressbook) -> Result<(), Error> {
        Err(Error::NotFound)
    }

    async fn delete_addressbook(
        &self,
        principal: &str,
        name: &str,
        use_trashbin: bool,
    ) -> Result<(), Error> {
        Err(Error::NotFound)
    }

    async fn restore_addressbook(&self, principal: &str, name: &str) -> Result<(), Error> {
        Err(Error::NotFound)
    }

    async fn sync_changes(
        &self,
        principal: &str,
        addressbook_id: &str,
        synctoken: i64,
    ) -> Result<(Vec<AddressObject>, Vec<String>, i64), Error> {
        Err(Error::NotFound)
    }

    async fn get_objects(
        &self,
        principal: &str,
        addressbook_id: &str,
    ) -> Result<Vec<AddressObject>, Error> {
        Err(Error::NotFound)
    }

    async fn get_object(
        &self,
        principal: &str,
        addressbook_id: &str,
        object_id: &str,
    ) -> Result<AddressObject, Error> {
        Err(Error::NotFound)
    }

    async fn put_object(
        &self,
        principal: String,
        addressbook_id: String,
        object: AddressObject,
        overwrite: bool,
    ) -> Result<(), Error> {
        Err(Error::NotFound)
    }

    async fn delete_object(
        &self,
        principal: &str,
        addressbook_id: &str,
        object_id: &str,
        use_trashbin: bool,
    ) -> Result<(), Error> {
        Err(Error::NotFound)
    }

    async fn restore_object(
        &self,
        principal: &str,
        addressbook_id: &str,
        object_id: &str,
    ) -> Result<(), Error> {
        Err(Error::NotFound)
    }
}

