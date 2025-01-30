use std::collections::HashMap;
use async_trait::async_trait;
use tracing::instrument;
use ical::parser::Component;
use rustical_store::calendar::{CalendarObjectComponent, EventObject};
use rustical_store::{auth::User, Calendar, CalendarObject, CalendarStore, Error};
use super::HulyStore;
use crate::api::{
    generate_id, tx_create_event, tx_delete_event, tx_update_event, HulyEvent, HulyEventCreateData, HulyEventUpdateData,
    CLASS_EVENT, CLASS_RECURRING_EVENT, CLASS_RECURRING_INSTANCE,
};
use crate::convert_time::from_ical_get_timestamp_required;

#[async_trait]
impl CalendarStore for HulyStore {
    #[instrument]
    async fn get_calendar(&self, user: &User, _: &str) -> Result<Calendar, Error> {
        tracing::debug!("GET_CALENDAR user={}, ws={:?}", user.id, user.workspace);
        let mut cache = self.calendar_cache.lock().await;
        let cal = cache.get_calendar(user).await?;
        Ok(cal)
    }

    #[instrument]
    async fn get_calendars(&self, user: &User) -> Result<Vec<Calendar>, Error> {
        tracing::debug!("GET_CALENDARS user={} ws={:?}", user.id, user.workspace);
        let mut cache = self.calendar_cache.lock().await;
        let cals = cache.get_calendars(user).await?;
        Ok(cals)
    }

    #[instrument]
    async fn get_deleted_calendars(&self, user: &str) -> Result<Vec<Calendar>, Error> {
        tracing::debug!("GET_DELETED_CALENDARS user={}", user);
        Ok(vec![])
    }

    #[instrument]
    async fn update_calendar(&self, user: String, cal_id: String, calendar: Calendar) -> Result<(), Error> {
        tracing::debug!("UPDATE_CALENDAR user={}, cal_id={}, calendar={:?}", user, cal_id, calendar);
        Err(Error::ApiError("not implemented".into()))
    }

    #[instrument]
    async fn insert_calendar(&self, calendar: Calendar) -> Result<(), Error> {
        tracing::debug!("INSERT_CALENDAR calendar={:?}", calendar);
        Err(Error::ApiError("not implemented".into()))
    }

    #[instrument]
    async fn delete_calendar(&self, user: &str, cal_id: &str, use_trashbin: bool) -> Result<(), Error> {
        tracing::debug!("DELETE_CALENDAR user={}, cal_id={}, use_trashbin={}", user, cal_id, use_trashbin);
        Err(Error::ApiError("not implemented".into()))
    }

    #[instrument]
    async fn restore_calendar(&self, user: &str, cal_id: &str) -> Result<(), Error> {
        tracing::debug!("RESTORE_CALENDAR user={}, cal_id={}", user, cal_id);
        Err(Error::ApiError("not implemented".into()))
    }

    #[instrument]
    async fn sync_changes(&self, user: &str, cal_id: &str, synctoken: i64) -> Result<(Vec<CalendarObject>, Vec<String>, i64), Error> {
        tracing::debug!("SYNC_CHANGES user={}, cal_id={}, synctoken={}", user, cal_id, synctoken);
        Err(Error::ApiError("not implemented".into()))
    }

    #[instrument]
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

    #[instrument]
    async fn get_object(&self, user: &User, _: &str, event_id: &str) -> Result<CalendarObject, Error> {
        tracing::debug!("GET_OBJECT user={}, ws={:?}, event={}", user.id, user.workspace, event_id);
        let mut cache = self.calendar_cache.lock().await;
        let event = cache.get_event(user, event_id, true).await?;
        let cal_obj: CalendarObject = event.try_into()?;
        println!("*** RETURN:\n{}", cal_obj.get_ics());
        Ok(cal_obj)
    }

    #[instrument]
    async fn put_object(&self, user: &User, _: String, object: CalendarObject, overwrite: bool) -> Result<(), Error> {
        tracing::debug!("PUT_OBJECT user={}, ws={:?}, object={:?}, overwrite={}", user.id, user.workspace, object, overwrite);
        println!("\n*** PUT_OBJECT:\n{}\n", object.get_ics());
        let event_id = object.id.as_str();
        if overwrite {
            match &object.data {
                CalendarObjectComponent::Event(event_obj) => {
                    self.update_event(user, event_id, event_obj).await
                }
                CalendarObjectComponent::Events(event_objs) => {
                    self.update_recurring_event(user, event_id, event_objs).await
                }
                _ => {
                    return Err(Error::InvalidData("invalid object type, must be event(s)".into()))
                }
            }
        } else {
            let CalendarObjectComponent::Event(event_obj) =  &object.data else {
                return Err(Error::InvalidData("invalid object type, must be event".into()))
            };
            self.create_event(user, event_id, event_obj).await
        }
    }

    #[instrument]
    async fn delete_object(&self, user: &User, _: &str, event_id: &str, use_trashbin: bool) -> Result<(), Error> {
        tracing::debug!("DELETE_OBJECT user={}, ws={:?}, event_id={:?}, use_trashbin={}", user.id, user.workspace, event_id, use_trashbin);
        let mut cache = self.calendar_cache.lock().await;
        let old_event = cache.get_event(user, event_id, true).await?;
        tx_delete_event(cache.api_url(), user, &old_event.data).await?;
        for instance in old_event.instances.as_ref().unwrap_or(&vec![]) {
            tx_delete_event(cache.api_url(), user, instance).await?;
        }
        cache.invalidate(user);
        Ok(())
    }

    #[instrument]
    async fn restore_object(&self, user: &str, cal_id: &str, object_id: &str) -> Result<(), Error> {
        tracing::debug!("RESTORE_OBJECT user={}, cal_id={}, object_id={}", user, cal_id, object_id);
        Err(Error::NotFound)
    }

    #[instrument]
    fn is_read_only(&self) -> bool {
        false
    }
}

impl HulyStore {
    async fn create_event(&self, user: &User, event_id: &str, event_obj: &EventObject) -> Result<(), Error> {
        let mut cache = self.calendar_cache.lock().await;
        let cal_id = cache.get_calendar_id(user).await?;
        let data = HulyEventCreateData::new(cal_id.as_str(), event_id, event_obj)?;
        // It's not the same id as Huly expect to display uset name in the event
        //data.participants = user.account.as_ref().map(|s| vec![s.clone()]);
        let class = if data.rules.is_some() { CLASS_RECURRING_EVENT } else { CLASS_EVENT };
        tx_create_event(cache.api_url(), user, class, &data).await?;
        cache.invalidate(user);
        Ok(())
    }

    async fn update_event(&self, user: &User, event_id: &str, event_obj: &EventObject) -> Result<(), Error> {
        let mut cache = self.calendar_cache.lock().await;
        let old_event = cache.get_event(user, event_id, false).await?;
        println!("*** OLD_EVENT:\n{}", old_event.pretty_str());
        let update_data = HulyEventUpdateData::new(&old_event.data, event_obj)?;
        if let Some(update_data) = update_data {
            tx_update_event(cache.api_url(), user, &old_event.data, &update_data).await?;
            cache.invalidate(user);
        }
        Ok(())
    }

    async fn update_recurring_event(&self, user: &User, event_id: &str, event_objs: &Vec<EventObject>) -> Result<(), Error> {
        let mut cache = self.calendar_cache.lock().await;
        let cal_id = cache.get_calendar_id(user).await?;
        let old_event = cache.get_event(user, event_id, true).await?;
        println!("*** OLD_RECURRING_EVENT:\n{}", old_event.pretty_str());
        for event_obj in event_objs {
            if event_obj.event.get_property("RECURRENCE-ID").is_some() {
                self.update_recurring_instance(user, cache.api_url(), cal_id.as_str(), &old_event, event_obj).await?;
            } else {
                // Update 'root' event (All instances)
                let update_data = HulyEventUpdateData::new(&old_event.data, event_obj)?;
                if let Some(update_data) = update_data {
                    tx_update_event(cache.api_url(), user, &old_event.data, &update_data).await?;
                }
            }
        }
        cache.invalidate(user);
        Ok(())
    }

    async fn update_recurring_instance(&self, user: &User, api_url: &str, cal_id: &str, old_event: &HulyEvent, event_obj: &EventObject) -> Result<(), Error> {
        let original_start_time = from_ical_get_timestamp_required(&event_obj.event, "RECURRENCE-ID")?;

        let old_instance = old_event.instances.as_ref().map(|instances| {
            instances.iter().find(|inst| inst.original_start_time.unwrap_or_default() == original_start_time)
        }).unwrap_or_default();

        if let Some(old_instance) = old_instance {
            let update_data = HulyEventUpdateData::new(old_instance, event_obj)?;
            if let Some(update_data) = update_data {
                tx_update_event(api_url, user, old_instance, &update_data).await?;
            }
        } else {
            let event_id = old_event.data.event_id.as_ref().
                ok_or_else(|| Error::InvalidData("Empty event id".into()))?;
            let instance_id = generate_id(api_url, &user.try_into()?, CLASS_RECURRING_INSTANCE).await?;
            let mut data = HulyEventCreateData::new(cal_id, instance_id.as_str(), event_obj)?;
            data.participants = Some(old_event.data.participants.clone());
            data.original_start_time = Some(original_start_time);
            data.recurring_event_id = Some(event_id.to_string());
            tx_create_event(api_url, user, CLASS_RECURRING_INSTANCE, &data).await?;
        };
        Ok(())
    }
}
