use std::collections::HashMap;
use async_trait::async_trait;
use tracing::instrument;
use ical::parser::Component;
use rustical_store::calendar::{CalendarObjectComponent, EventObject};
use rustical_store::{auth::User, Calendar, CalendarObject, CalendarStore, Error};
use super::HulyStore;
use crate::api::{
    generate_id, tx_create_event, tx_delete_event, tx_update_event, HulyEvent, HulyEventData, HulyEventCreateData, HulyEventUpdateData,
    CLASS_EVENT, CLASS_RECURRING_EVENT, CLASS_RECURRING_INSTANCE,
};
use crate::convert::from_ical_get_alarms;
use crate::convert_rrule::parse_rrule_string;
use crate::convert_time::{from_ical_get_event_bounds, from_ical_get_timestamp_required, from_ical_get_exdate, from_ical_get_timezone};

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
        let class = if data.rules.is_some() { CLASS_RECURRING_EVENT } else { CLASS_EVENT };
        tx_create_event(cache.api_url(), user, class, &data).await?;
        cache.invalidate(user);
        Ok(())
    }

    async fn update_event_raw(&self, user: &User, api_url: &str, old_event: &HulyEventData, event_obj: &EventObject) -> Result<(), Error> {
        let mut update_data = HulyEventUpdateData::default();
        let mut update_count = 0;
        if let Some(prop) = event_obj.event.get_property("SUMMARY") {
            if let Some(value) = &prop.value {
                if *value != old_event.title {
                    update_data.title = Some(value.clone());
                    update_count += 1;
                }
            }
        }
        // TODO: handle markdown
        // if let Some(prop) = ical_event.get_property("DESCRIPTION") {
        //     if let Some(value) = &prop.value {
        //         if *value != old_event.description {
        //             updates.description = Some(value.clone());
        //             update_count += 1;
        //         }
        //     }
        // }
        if let Some(prop) = event_obj.event.get_property("LOCATION") {
            if let Some(value) = &prop.value {
                if let Some(old_value) = &old_event.location {
                    if value != old_value {
                        update_data.location = Some(value.to_string());
                        update_count += 1;
                    }
                } else {
                    update_data.location = Some(value.to_string());
                    update_count += 1;
                }
            }
        }
        let (date, due_date, all_day) = from_ical_get_event_bounds(&event_obj.event)?;
        if date != old_event.date {
            update_data.date = Some(date);
            update_count += 1;
        }
        if due_date != old_event.due_date {
            update_data.due_date = Some(due_date);
            update_count += 1;
        }
        if all_day != old_event.all_day {
            update_data.all_day = Some(all_day);
            update_count += 1;
        }
        let reminders = from_ical_get_alarms(&event_obj.event)?;
        if reminders != old_event.reminders {
            update_data.reminders = reminders;
            update_count += 1;
        }

        // There is no direct way in Huly to change event recurrency
        // ReccuringEvent is a different object class and must be recreated
        let is_old_recurrent = old_event.rules.as_ref().and_then(|r| Some(!r.is_empty())).unwrap_or(false);
        if let Some(prop) = event_obj.event.get_property("RRULE") {
            if let Some(value) = &prop.value {
                if !is_old_recurrent {
                    return Err(Error::InvalidData("Unable change event recurrency".into()));
                }
                let rules = parse_rrule_string(value.as_str())?;
                if old_event.rules != rules {
                    update_data.rules = rules;
                    update_count += 1;
                }
            } else if is_old_recurrent {
                return Err(Error::InvalidData("Unable change event recurrency".into()));
            }
        } else if is_old_recurrent {
            return Err(Error::InvalidData("Unable change event recurrency".into()));
        }
        if is_old_recurrent {
            let exdate = from_ical_get_exdate(&event_obj.event)?;
            if old_event.exdate != exdate {
                update_data.exdate = exdate;
                update_count += 1;
            }
        }

        let time_zone = from_ical_get_timezone(event_obj)?;
        if let Some(time_zone) = time_zone {
            if let Some(old_time_zone) = &old_event.time_zone {
                if &time_zone != old_time_zone {
                    update_data.time_zone = Some(time_zone);
                    update_count += 1;
                }
            } else {
                update_data.time_zone = Some(time_zone);
                update_count += 1;
            }
        } else if old_event.time_zone.is_some() {
            update_data.time_zone = None;
            update_count += 1;
        }

        // TODO: handle attachments

        if update_count == 0 {
            return Ok(());
        }

        tx_update_event(api_url, user, old_event, &update_data).await
    }

    async fn update_event(&self, user: &User, event_id: &str, event_obj: &EventObject) -> Result<(), Error> {
        let mut cache = self.calendar_cache.lock().await;
        let old_event = cache.get_event(user, event_id, false).await?;
        println!("*** OLD_EVENT:\n{}", old_event.pretty_str());
        self.update_event_raw(user, cache.api_url(), &old_event.data, event_obj).await?;
        cache.invalidate(user);
        Ok(())
    }

    async fn create_recurring_instance(&self, user: &User, api_url: &str, cal_id: &str, event_id: &str, event_obj: &EventObject) -> Result<(), Error> {
        let instance_id = generate_id(api_url, &user.try_into()?, CLASS_RECURRING_INSTANCE).await?;
        let mut data = HulyEventCreateData::new(cal_id, instance_id.as_str(), event_obj)?;
        let original_start_time = from_ical_get_timestamp_required(&event_obj.event, "RECURRENCE-ID")?;
        data.original_start_time = Some(original_start_time);
        data.recurring_event_id = Some(event_id.to_string());
        tx_create_event(api_url, user, CLASS_RECURRING_INSTANCE, &data).await
    }

    async fn update_recurring_instance(&self, user: &User, api_url: &str, cal_id: &str, old_event: &HulyEvent, event_obj: &EventObject) -> Result<(), Error> {
        let original_start_time = from_ical_get_timestamp_required(&event_obj.event, "RECURRENCE-ID")?;
        let old_instances = old_event.instances.as_ref()
            .ok_or_else(|| Error::InvalidData("Empty recurring instances".into()))?;
        let old_instance = old_instances.iter()
            .find(|inst| inst.original_start_time.unwrap_or_default() == original_start_time);
        if let Some(old_instance) = old_instance {
            self.update_event_raw(user, api_url, old_instance, event_obj).await
        } else {
            let event_id = old_event.data.event_id.as_ref().
                ok_or_else(|| Error::InvalidData("Empty event id".into()))?;
            self.create_recurring_instance(user, api_url, cal_id, event_id.as_str(), event_obj).await
        }
    }

    async fn update_recurring_event(&self, user: &User, event_id: &str, event_objs: &Vec<EventObject>) -> Result<(), Error> {
        let mut cache = self.calendar_cache.lock().await;
        let cal_id = cache.get_calendar_id(user).await?;
        let old_event = cache.get_event(user, event_id, true).await?;
        println!("*** OLD_RECURRING_EVENT:\n{}", old_event.pretty_str());
        for event_obj in event_objs {
            if event_obj.event.get_property("RECURRENCE-ID").is_some() {
                if old_event.instances.is_none() {
                    self.create_recurring_instance(user, cache.api_url(), cal_id.as_str(), event_id, event_obj).await?;
                } else {
                    self.update_recurring_instance(user, cache.api_url(), cal_id.as_str(), &old_event, event_obj).await?;
                }
            } else {
                self.update_event_raw(user, cache.api_url(), &old_event.data, event_obj).await?;
            }
        }
        cache.invalidate(user);
        Ok(())
    }
}
