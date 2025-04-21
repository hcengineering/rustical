use super::HulyStore;
use crate::api::{
    CLASS_EVENT, CLASS_RECURRING_EVENT, CLASS_RECURRING_INSTANCE, HulyEvent, HulyEventCreateData,
    HulyEventUpdateData, generate_id, tx_create_event, tx_delete_event, tx_update_event,
};
use crate::auth::HulyUser;
use crate::convert_time::from_ical_get_timestamp_required;
use async_trait::async_trait;
use ical::parser::Component;
use rustical_store::calendar::{CalendarObjectComponent, EventObject};
use rustical_store::{Calendar, CalendarObject, CalendarStore, Error};
use std::collections::HashMap;
use tracing::instrument;

#[async_trait]
impl CalendarStore for HulyStore {
    #[instrument]
    async fn get_calendar(&self, user_id: &str, ws_url: &str) -> Result<Calendar, Error> {
        println!("GET_CALENDAR user_id={}, ws_url={}", user_id, ws_url);
        let mut cache = self.calendar_cache.lock().await;
        let user = cache.get_user(user_id, Some(ws_url))?;
        let cal = cache.get_calendar(&user).await?;
        Ok(cal)
    }

    #[instrument]
    async fn get_calendars(&self, user_id: &str) -> Result<Vec<Calendar>, Error> {
        println!("GET_CALENDARS user_id={}", user_id);
        let mut cache = self.calendar_cache.lock().await;
        let user = cache.get_user(user_id, None)?;
        let cals = cache.get_calendars(&user).await?;
        Ok(cals)
    }

    #[instrument]
    async fn get_deleted_calendars(&self, user_id: &str) -> Result<Vec<Calendar>, Error> {
        println!("GET_DELETED_CALENDARS user_id={}", user_id);
        Ok(vec![])
    }

    #[instrument]
    async fn update_calendar(
        &self,
        user_id: String,
        ws_url: String,
        calendar: Calendar,
    ) -> Result<(), Error> {
        println!(
            "UPDATE_CALENDAR user_id={}, ws_url={}, calendar={:?}",
            user_id, ws_url, calendar
        );
        Ok(())
    }

    #[instrument]
    async fn insert_calendar(&self, calendar: Calendar) -> Result<(), Error> {
        println!("INSERT_CALENDAR calendar={:?}", calendar);
        Err(Error::ApiError("not implemented".into()))
    }

    #[instrument]
    async fn delete_calendar(
        &self,
        user_id: &str,
        ws_url: &str,
        use_trashbin: bool,
    ) -> Result<(), Error> {
        println!(
            "DELETE_CALENDAR user_id={}, ws_url={}, use_trashbin={}",
            user_id, ws_url, use_trashbin
        );
        Err(Error::ApiError("not implemented".into()))
    }

    #[instrument]
    async fn restore_calendar(&self, user_id: &str, ws_url: &str) -> Result<(), Error> {
        println!("RESTORE_CALENDAR user_id={}, ws_url={}", user_id, ws_url);
        Err(Error::ApiError("not implemented".into()))
    }

    #[instrument]
    async fn sync_changes(
        &self,
        user_id: &str,
        ws_url: &str,
        synctoken: i64,
    ) -> Result<(Vec<CalendarObject>, Vec<String>, i64), Error> {
        println!(
            "SYNC_CHANGES user_id={}, ws_url={}, synctoken={}",
            user_id, ws_url, synctoken
        );
        let mut cache = self.calendar_cache.lock().await;
        let user = cache.get_user(user_id, Some(ws_url))?;
        let events = cache.get_events(&user).await?;
        let cur_synctoken = cache.get_synctoken(&user).await?;
        println!("cur_synctoken={}", cur_synctoken);

        // If the synctoken is the same as the new one, no changes
        if synctoken == cur_synctoken {
            return Ok((vec![], vec![], cur_synctoken));
        }

        // Get the previous sync state if available
        let previous_event_ids = if synctoken > 0 {
            cache
                .get_sync_state(synctoken as u64)
                .await
                .unwrap_or_default()
        } else {
            vec![]
        };

        // Get the current event IDs
        let current_event_ids: Vec<String> = events.iter().map(|(id, _)| id.clone()).collect();

        // Store the current sync state
        cache
            .set_sync_state(cur_synctoken as u64, current_event_ids.clone())
            .await?;

        // Calculate added and removed events
        let mut added_events = Vec::new();
        let mut removed_event_ids = Vec::new();

        // Find added events
        for (id, etag) in &events {
            if !previous_event_ids.contains(id) {
                added_events.push(CalendarObject {
                    id: id.clone(),
                    ics: "".to_string(),
                    etag: Some(etag.clone()),
                    data: CalendarObjectComponent::Event(EventObject {
                        event: ical::generator::IcalEvent::default(),
                        timezones: HashMap::new(),
                    }),
                });
            }
        }

        // Find removed events
        for id in &previous_event_ids {
            if !current_event_ids.contains(id) {
                removed_event_ids.push(id.clone());
            }
        }

        // If this is the first sync, return all events
        if synctoken <= 0 {
            let cal_objs = events
                .into_iter()
                .map(|(id, etag)| CalendarObject {
                    id,
                    ics: "".to_string(),
                    etag: Some(etag),
                    data: CalendarObjectComponent::Event(EventObject {
                        event: ical::generator::IcalEvent::default(),
                        timezones: HashMap::new(),
                    }),
                })
                .collect();
            return Ok((cal_objs, vec![], cur_synctoken));
        }

        Ok((added_events, removed_event_ids, cur_synctoken))
    }

    #[instrument]
    async fn get_objects(&self, user_id: &str, ws_url: &str) -> Result<Vec<CalendarObject>, Error> {
        println!("GET_OBJECTS user_id={}, ws_url={}", user_id, ws_url);
        let mut cache = self.calendar_cache.lock().await;
        let user = cache.get_user(user_id, Some(ws_url))?;
        let cur_synctoken = cache.get_synctoken(&user).await?;
        println!("cur_synctoken={}", cur_synctoken);
        let events = cache.get_events(&user).await?;
        // Calendar app uses this request only for getting event etags
        // Then it compares them with the etags it already has,
        // and queries full objects via separate calls by ID.
        // So that's why we return here empty ics and data
        let cal_objs = events
            .into_iter()
            .map(|(id, etag)| CalendarObject {
                id,
                ics: "".to_string(),
                etag: Some(etag),
                data: CalendarObjectComponent::Event(EventObject {
                    event: ical::generator::IcalEvent::default(),
                    timezones: HashMap::new(),
                }),
            })
            .collect();

        Ok(cal_objs)
    }

    #[instrument]
    async fn get_object(
        &self,
        user_id: &str,
        ws_url: &str,
        event_id: &str,
    ) -> Result<CalendarObject, Error> {
        println!(
            "GET_OBJECT user_id={}, ws_url={}, event_id={}",
            user_id, ws_url, event_id
        );
        let mut cache = self.calendar_cache.lock().await;
        let user = cache.get_user(user_id, Some(ws_url))?;
        let event = cache.get_event(&user, event_id, true).await?;
        let cal_obj: CalendarObject = event.try_into()?;
        //println!("*** RETURN:\n{}", cal_obj.get_ics());
        Ok(cal_obj)
    }

    #[instrument]
    async fn put_object(
        &self,
        user_id: String,
        ws_url: String,
        object: CalendarObject,
        overwrite: bool,
    ) -> Result<(), Error> {
        println!(
            "PUT_OBJECT user_id={}, ws_url={}, overwrite={}\n{}",
            user_id,
            ws_url,
            overwrite,
            object.get_ics(),
        );
        let event_id = object.id.as_str();
        if overwrite {
            match &object.data {
                CalendarObjectComponent::Event(event_obj) => {
                    self.update_event(&user_id, &ws_url, event_id, event_obj)
                        .await
                }
                CalendarObjectComponent::Events(event_objs) => {
                    self.update_recurring_event(&user_id, &ws_url, event_id, event_objs)
                        .await
                }
                _ => {
                    return Err(Error::InvalidData(
                        "invalid object type, must be event(s)".into(),
                    ));
                }
            }
        } else {
            let CalendarObjectComponent::Event(event_obj) = &object.data else {
                return Err(Error::InvalidData(
                    "invalid object type, must be event".into(),
                ));
            };
            self.create_event(&user_id, &ws_url, event_id, event_obj)
                .await
        }
    }

    #[instrument]
    async fn delete_object(
        &self,
        user_id: &str,
        ws_url: &str,
        event_id: &str,
        use_trashbin: bool,
    ) -> Result<(), Error> {
        println!(
            "DELETE_OBJECT user_id={}, ws_url={}, event_id={:?}, use_trashbin={}",
            user_id, ws_url, event_id, use_trashbin
        );
        let mut cache = self.calendar_cache.lock().await;
        let user = cache.get_user(user_id, Some(ws_url))?;
        let old_event = cache.get_event(&user, event_id, true).await?;
        tx_delete_event(&user, &old_event.data).await?;
        for instance in old_event.instances.as_ref().unwrap_or(&vec![]) {
            tx_delete_event(&user, instance).await?;
        }
        cache.invalidate(&user);
        Ok(())
    }

    #[instrument]
    async fn restore_object(
        &self,
        user_id: &str,
        ws_url: &str,
        object_id: &str,
    ) -> Result<(), Error> {
        println!(
            "RESTORE_OBJECT user_id={}, ws_url={}, object_id={}",
            user_id, ws_url, object_id
        );
        Err(Error::NotFound)
    }

    #[instrument]
    fn is_read_only(&self) -> bool {
        false
    }
}

impl HulyStore {
    async fn create_event(
        &self,
        user_id: &str,
        ws_url: &str,
        event_id: &str,
        event_obj: &EventObject,
    ) -> Result<(), Error> {
        let mut cache = self.calendar_cache.lock().await;
        let user = cache.get_user(user_id, Some(ws_url))?;
        let cal_id = cache.get_calendar_id(&user).await?;
        let data = HulyEventCreateData::new(&user, cal_id.as_str(), event_id, event_obj)?;
        // It's not the same id as Huly expect to display uset name in the event
        //data.participants = user.account.as_ref().map(|s| vec![s.clone()]);
        let class = if data.rules.is_some() {
            CLASS_RECURRING_EVENT
        } else {
            CLASS_EVENT
        };
        tx_create_event(&user, class, &data).await?;
        cache.invalidate(&user);
        Ok(())
    }

    async fn update_event(
        &self,
        user_id: &str,
        ws_url: &str,
        event_id: &str,
        event_obj: &EventObject,
    ) -> Result<(), Error> {
        let mut cache = self.calendar_cache.lock().await;
        let user = cache.get_user(user_id, Some(ws_url))?;
        let old_event = cache.get_event(&user, event_id, false).await?;
        //println!("*** OLD_EVENT:\n{}", old_event.pretty_str());
        let update_data = HulyEventUpdateData::new(&old_event.data, event_obj)?;
        if let Some(update_data) = update_data {
            tx_update_event(&user, &old_event.data, &update_data).await?;
            cache.invalidate(&user);
        }
        Ok(())
    }

    async fn update_recurring_event(
        &self,
        user_id: &str,
        ws_url: &str,
        event_id: &str,
        event_objs: &Vec<EventObject>,
    ) -> Result<(), Error> {
        let mut cache = self.calendar_cache.lock().await;
        let user = cache.get_user(user_id, Some(ws_url))?;
        let cal_id = cache.get_calendar_id(&user).await?;
        let old_event = cache.get_event(&user, event_id, true).await?;
        //println!("*** OLD_RECURRING_EVENT:\n{}", old_event.pretty_str());
        for event_obj in event_objs {
            if event_obj.event.get_property("RECURRENCE-ID").is_some() {
                self.update_recurring_instance(&user, cal_id.as_str(), &old_event, event_obj)
                    .await?;
            } else {
                // Update 'root' event (All instances)
                let update_data = HulyEventUpdateData::new(&old_event.data, event_obj)?;
                if let Some(update_data) = update_data {
                    tx_update_event(&user, &old_event.data, &update_data).await?;
                }
            }
        }
        cache.invalidate(&user);
        Ok(())
    }

    async fn update_recurring_instance(
        &self,
        user: &HulyUser,
        cal_id: &str,
        old_event: &HulyEvent,
        event_obj: &EventObject,
    ) -> Result<(), Error> {
        let original_start_time =
            from_ical_get_timestamp_required(&event_obj.event, "RECURRENCE-ID")?;

        let old_instance = old_event
            .instances
            .as_ref()
            .map(|instances| {
                instances.iter().find(|inst| {
                    inst.original_start_time.unwrap_or_default() == original_start_time
                })
            })
            .unwrap_or_default();

        if let Some(old_instance) = old_instance {
            let update_data = HulyEventUpdateData::new(old_instance, event_obj)?;
            if let Some(update_data) = update_data {
                tx_update_event(user, old_instance, &update_data).await?;
            }
        } else {
            let event_id = old_event
                .data
                .event_id
                .as_ref()
                .ok_or_else(|| Error::InvalidData("Empty event id".into()))?;
            let instance_id = generate_id(user).await?;
            let mut data =
                HulyEventCreateData::new(&user, cal_id, instance_id.as_str(), event_obj)?;
            data.participants = Some(old_event.data.participants.clone());
            data.original_start_time = Some(original_start_time);
            data.recurring_event_id = Some(event_id.to_string());
            tx_create_event(user, CLASS_RECURRING_INSTANCE, &data).await?;
        };
        Ok(())
    }
}
