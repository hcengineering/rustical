use std::collections::HashMap;

use async_trait::async_trait;
use rustical_store::calendar::{CalendarObjectComponent, EventObject};
use rustical_store::{auth::User, Calendar, CalendarObject, CalendarStore, Error};
use super::HulyStore;
use crate::api::{generate_id, tx, HulyEventCreateData, HulyEventUpdateData, HulyEventTx, Timestamp};
use crate::auth::get_workspaces;
use crate::convert::{parse_to_utc_msec, parse_rrule_string};

#[async_trait]
impl CalendarStore for HulyStore {
    async fn get_calendar(&self, user: &User, _: &str) -> Result<Calendar, Error> {
        tracing::debug!("GET_CALENDAR user={}, ws={:?}", user.id, user.workspace);
        let mut cache = self.calendar_cache.lock().await;
        let cal = cache.get_calendar(user).await?;
        Ok(cal)
    }

    async fn get_calendars(&self, user: &User) -> Result<Vec<Calendar>, Error> {
        tracing::debug!("GET_CALENDARS user={} ws={:?}", user.id, user.workspace);
        let mut cache = self.calendar_cache.lock().await;
        let cals = cache.get_calendars(user).await?;
        Ok(cals)
    }

    async fn get_deleted_calendars(&self, user: &str) -> Result<Vec<Calendar>, Error> {
        tracing::debug!("GET_DELETED_CALENDARS user={}", user);
        Ok(vec![])
    }

    async fn update_calendar(&self, user: String, cal_id: String, calendar: Calendar) -> Result<(), Error> {
        tracing::debug!("UPDATE_CALENDAR user={}, cal_id={}, calendar={:?}", user, cal_id, calendar);
        Err(Error::ApiError("not implemented".into()))
    }

    async fn insert_calendar(&self, calendar: Calendar) -> Result<(), Error> {
        tracing::debug!("INSERT_CALENDAR calendar={:?}", calendar);
        Err(Error::ApiError("not implemented".into()))
    }

    async fn delete_calendar(&self, user: &str, cal_id: &str, use_trashbin: bool) -> Result<(), Error> {
        tracing::debug!("DELETE_CALENDAR user={}, cal_id={}, use_trashbin={}", user, cal_id, use_trashbin);
        Err(Error::ApiError("not implemented".into()))
    }

    async fn restore_calendar(&self, user: &str, cal_id: &str) -> Result<(), Error> {
        tracing::debug!("RESTORE_CALENDAR user={}, cal_id={}", user, cal_id);
        Err(Error::ApiError("not implemented".into()))
    }

    async fn sync_changes(&self, user: &str, cal_id: &str, synctoken: i64) -> Result<(Vec<CalendarObject>, Vec<String>, i64), Error> {
        tracing::debug!("SYNC_CHANGES user={}, cal_id={}, synctoken={}", user, cal_id, synctoken);
        Err(Error::ApiError("not implemented".into()))
    }

    async fn get_objects(&self, user: &User, _: &str) -> Result<Vec<CalendarObject>, Error> {
        tracing::debug!("GET_OBJECTS user={}, ws={:?}", user.id, user.workspace);
        let mut cache = self.calendar_cache.lock().await;
        let events = cache.get_events(user).await?;
        let cal_objs = events.into_iter().map(|(id, etag)| CalendarObject {
            id,
            ics: "".to_string(),
            etag: Some(etag),
            data: CalendarObjectComponent::Event(EventObject {
                event: ical::generator::IcalEvent::default(),
                timezones: HashMap::new(),
            })
        }).collect();
        Ok(cal_objs)
    }

    async fn get_object(&self, user: &User, _: &str, event_id: &str) -> Result<CalendarObject, Error> {
        tracing::debug!("GET_OBJECT user={}, ws={:?}, event={}", user.id, user.workspace, event_id);
        let mut cache = self.calendar_cache.lock().await;
        let event = cache.get_event(user, event_id).await?;
        let cal_obj: CalendarObject = event.try_into()?;
        Ok(cal_obj)
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
