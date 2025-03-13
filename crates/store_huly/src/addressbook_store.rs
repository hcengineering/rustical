use super::HulyStore;
use async_trait::async_trait;
use rustical_store::{AddressObject, Addressbook, AddressbookStore, Error};

#[async_trait]
impl AddressbookStore for HulyStore {
    async fn get_addressbook(&self, _principal: &str, _id: &str) -> Result<Addressbook, Error> {
        Err(Error::NotFound)
    }

    async fn get_addressbooks(&self, _principal: &str) -> Result<Vec<Addressbook>, Error> {
        Err(Error::NotFound)
    }

    async fn get_deleted_addressbooks(&self, _principal: &str) -> Result<Vec<Addressbook>, Error> {
        Err(Error::NotFound)
    }

    async fn update_addressbook(
        &self,
        _principal: String,
        _id: String,
        _addressbook: Addressbook,
    ) -> Result<(), Error> {
        Err(Error::NotFound)
    }

    async fn insert_addressbook(&self, _addressbook: Addressbook) -> Result<(), Error> {
        Err(Error::NotFound)
    }

    async fn delete_addressbook(
        &self,
        _principal: &str,
        _name: &str,
        _use_trashbin: bool,
    ) -> Result<(), Error> {
        Err(Error::NotFound)
    }

    async fn restore_addressbook(&self, _principal: &str, _name: &str) -> Result<(), Error> {
        Err(Error::NotFound)
    }

    async fn sync_changes(
        &self,
        _principal: &str,
        _addressbook_id: &str,
        _synctoken: i64,
    ) -> Result<(Vec<AddressObject>, Vec<String>, i64), Error> {
        Err(Error::NotFound)
    }

    async fn get_objects(
        &self,
        _principal: &str,
        _addressbook_id: &str,
    ) -> Result<Vec<AddressObject>, Error> {
        Err(Error::NotFound)
    }

    async fn get_object(
        &self,
        _principal: &str,
        _addressbook_id: &str,
        _object_id: &str,
    ) -> Result<AddressObject, Error> {
        Err(Error::NotFound)
    }

    async fn put_object(
        &self,
        _principal: String,
        _addressbook_id: String,
        _object: AddressObject,
        _overwrite: bool,
    ) -> Result<(), Error> {
        Err(Error::NotFound)
    }

    async fn delete_object(
        &self,
        _principal: &str,
        _addressbook_id: &str,
        _object_id: &str,
        _use_trashbin: bool,
    ) -> Result<(), Error> {
        Err(Error::NotFound)
    }

    async fn restore_object(
        &self,
        _principal: &str,
        _addressbook_id: &str,
        _object_id: &str,
    ) -> Result<(), Error> {
        Err(Error::NotFound)
    }
}
