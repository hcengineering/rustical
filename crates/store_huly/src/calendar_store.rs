use async_trait::async_trait;
use rustical_store::{auth::User, Calendar, CalendarObject, CalendarStore, Error};
use super::HulyStore;

#[async_trait]
impl CalendarStore for HulyStore {
    async fn get_calendar(&self, principal: &User, id: &str) -> Result<Calendar, Error> {
        Err(Error::NotFound)
    }

    async fn get_calendars(&self, principal: &User) -> Result<Vec<Calendar>, Error> {
        Err(Error::NotFound)
    }

    async fn get_deleted_calendars(&self, principal: &str) -> Result<Vec<Calendar>, Error> {
        Err(Error::NotFound)
    }

    async fn update_calendar(
        &self,
        principal: String,
        id: String,
        calendar: Calendar,
    ) -> Result<(), Error> {
        Err(Error::NotFound)
    }

    async fn insert_calendar(&self, calendar: Calendar) -> Result<(), Error> {
        Err(Error::NotFound)
    }

    async fn delete_calendar(
        &self,
        principal: &str,
        name: &str,
        use_trashbin: bool,
    ) -> Result<(), Error> {
        Err(Error::NotFound)
    }

    async fn restore_calendar(&self, principal: &str, name: &str) -> Result<(), Error> {
        Err(Error::NotFound)
    }

    async fn sync_changes(
        &self,
        principal: &str,
        cal_id: &str,
        synctoken: i64,
    ) -> Result<(Vec<CalendarObject>, Vec<String>, i64), Error> {
        Err(Error::NotFound)
    }

    async fn get_objects(
        &self,
        principal: &User,
        cal_id: &str,
    ) -> Result<Vec<CalendarObject>, Error> {
        Err(Error::NotFound)
    }

    async fn get_object(
        &self,
        principal: &User,
        cal_id: &str,
        object_id: &str,
    ) -> Result<CalendarObject, Error> {
        Err(Error::NotFound)
    }

    async fn put_object(
        &self,
        principal: &User,
        cal_id: String,
        object: CalendarObject,
        overwrite: bool,
    ) -> Result<(), Error> {
        Err(Error::NotFound)
    }

    async fn delete_object(
        &self,
        principal: &User,
        cal_id: &str,
        object_id: &str,
        use_trashbin: bool,
    ) -> Result<(), Error> {
        Err(Error::NotFound)
    }

    async fn restore_object(
        &self,
        principal: &str,
        cal_id: &str,
        object_id: &str,
    ) -> Result<(), Error> {
        Err(Error::NotFound)
    }

    fn is_read_only(&self) -> bool {
        false
    }
}
