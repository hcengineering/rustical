use std::collections::HashMap;
use std::str::FromStr;
use async_trait::async_trait;
use tracing::instrument;
use ical::{generator::IcalEvent, parser::Component};
use rustical_store::calendar::{CalendarObjectComponent, EventObject};
use rustical_store::{auth::User, Calendar, CalendarObject, CalendarStore, Error};
use super::HulyStore;
use crate::api::{generate_id, tx, HulyEventCreateData, HulyEventUpdateData, HulyEventTx, Timestamp};
use crate::convert::{parse_to_utc_msec, parse_rrule_string};

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
        let event = cache.get_event(user, event_id).await?;
        let cal_obj: CalendarObject = event.try_into()?;
        Ok(cal_obj)
    }

    #[instrument]
    async fn put_object(&self, user: &User, _: String, object: CalendarObject, overwrite: bool) -> Result<(), Error> {
        tracing::debug!("PUT_OBJECT user={}, ws={:?}, object={:?}, overwrite={}", user.id, user.workspace, object, overwrite);
        println!("{}", object.get_ics());
        let CalendarObjectComponent::Event(ical_event) =  &object.data else {
            return Err(Error::InvalidData("invalid object type, must be event".into()))
        };
        if overwrite {
            self.update_event(user, object.id.as_str(), ical_event).await
        } else {
            self.create_event(user, object.id.as_str(), ical_event).await
        }
    }

    #[instrument]
    async fn delete_object(&self, user: &User, _: &str, event_id: &str, use_trashbin: bool) -> Result<(), Error> {
        tracing::debug!("DELETE_OBJECT user={}, ws={:?}, event_id={:?}, use_trashbin={}", user.id, user.workspace, event_id, use_trashbin);
        let mut cache = self.calendar_cache.lock().await;
        let old_event = cache.get_event(user, event_id).await?;
        println!("*** old_event: {}", serde_json::to_string_pretty(&old_event).unwrap());

        let tx_id = generate_id(cache.api_url(), user.try_into()?, "core:class:TxUpdateDoc").await?;

        let remove_tx = HulyEventTx::<()> {
            id: tx_id,
            class: "core:class:TxRemoveDoc".to_string(),
            space: "core:space:Tx".to_string(),
            // TODO: extract user ids from workspace token
            modified_by: old_event.modified_by.clone(),
            created_by: old_event.modified_by.clone(),
            object_id: old_event.id.clone(),
            object_class: old_event.class.clone(),
            object_space: old_event.space.clone(),
            operations: None,
            attributes: None,
            collection: old_event.collection.clone(),
            attached_to: old_event.attached_to.clone(),
            attached_to_class: old_event.attached_to_class.clone(),
        };

        println!("*** REMOVE TX {}", serde_json::to_string_pretty(&remove_tx).unwrap());

        tx(cache.api_url(), user.try_into()?, remove_tx).await?;
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
    fn get_timestamp(&self, prop: &ical::property::Property, prop_name: &str) -> Result<(Option<Timestamp>, bool), Error> {
        let Some(value) = &prop.value else {
            return Ok((None, false))
        };
        let Some(params) = &prop.params else {
            return Ok((None, false))
        };
        for (param_name, param_values) in params {
            match param_name.as_str() {
                // params=Some([("VALUE", ["DATE"])]),
                "VALUE" => {
                    if param_values.contains(&"DATE".to_string()) {
                        let local = chrono::NaiveDate::parse_from_str(value.as_str(), "%Y%m%d");
                        if let Err(err) = local {
                            return Err(Error::InvalidData(format!("invalid date: {}: {}", prop_name, err)));
                        }
                        let local = local.unwrap();
                        let Some(dt) = local.and_hms_opt(0, 0, 0) else {
                            return Err(Error::InvalidData(format!("invalid date-time: {}", prop_name)));
                        };
                        let ms = dt.and_utc().timestamp_millis();
                        return Ok((Some(ms), true));
                    }
                },
                // params=Some([("TZID", ["Asia/Novosibirsk"])]), 
                "TZID" => {
                    if param_values.is_empty() {
                        return Err(Error::InvalidData(format!("timezone not set: {}", prop_name)));
                    }
                    let tzid = param_values[0].as_str();
                    let tz = chrono_tz::Tz::from_str(tzid);
                    if let Err(err) = tz {
                        return Err(Error::InvalidData(format!("invalid timezone: {}: {}", prop_name, err)));
                    }
                    let tz = tz.unwrap();
                    let ms = parse_to_utc_msec(value.as_str(), &tz, prop_name)?;
                    return Ok((Some(ms), false));
                },
                _ => {},
            }
        }
        return Ok((None, false))
    }

    fn get_timestamps(&self, event: &IcalEvent) -> Result<(Option<Timestamp>, Option<Timestamp>, bool), Error> {
        let (start, all_day_1) = if let Some(prop) = event.get_property("DTSTART") {
            self.get_timestamp(prop, "DTSTART")?
        } else {
            (None, false)
        };
        let (end, all_day_2) = if let Some(prop) = event.get_property("DTEND") {
            self.get_timestamp(prop, "DTEND")?
        } else {
            (None, false)
        };
        // TODO: handle DURATION property
        // RFC5545: In a "VEVENT" calendar component the property may be
        // used to specify a duration of the event, instead of an explicit end DATE-TIME
        let all_day = all_day_1 && all_day_2;
        if all_day {
            if let Some(utc_ms) = end {
                // Huly defines all-day event as date={start_day}{00:00:00} due_date={end_day}{23:59:59:999}
                // While CaldDav clients sends DTEND={end_day+1}{no time part}
                // Shifting end date by 1 ms, we switch to the last ms of the prev day
                return Ok((start, Some(utc_ms-1), true));
            }
        }
        Ok((start, end, all_day))
    }

    fn get_timezone(&self, ical_event: &EventObject) -> Result<Option<String>, Error> {
        if ical_event.timezones.len() > 1 {
            return Err(Error::InvalidData("multiple timezones not supported".into()))
        }
        let tzids: Vec<String> = ical_event.timezones.keys().cloned().collect();
        Ok(tzids.first().cloned())
    }

    fn get_exdate(&self, ical_event: &EventObject) -> Result<Option<Vec<Timestamp>>, Error> {
        let mut exdate = Vec::new();
            for prop in &ical_event.event.properties {
                if prop.name == "EXDATE" {
                    let (dt, _) = self.get_timestamp(prop, "EXDATE")?;
                    if let Some(dt) = dt {
                        exdate.push(dt);
                    }
                }
            }
        Ok(if exdate.is_empty() {
            None
        } else {
            Some(exdate)
        })
    }

    async fn update_event(&self, user: &User, event_id: &str, ical_event: &EventObject) -> Result<(), Error> {
        let mut cache = self.calendar_cache.lock().await;
        let old_event = cache.get_event(user, event_id).await?;
        println!("*** old_event: {}", serde_json::to_string_pretty(&old_event).unwrap());
        let mut update_data = HulyEventUpdateData::default();
        let mut update_count = 0;
        if let Some(prop) = ical_event.event.get_property("SUMMARY") {
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
        if let Some(prop) = ical_event.event.get_property("LOCATION") {
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
        let (start, end, all_day) = self.get_timestamps(&ical_event.event)?;
        if let Some(utc_msec) = start {
            if utc_msec != old_event.date {
                update_data.date = Some(utc_msec);
                update_count += 1;
            }
        }
        if let Some(utc_msec) = end {
            if utc_msec != old_event.due_date {
                update_data.due_date = Some(utc_msec);
                update_count += 1;
            }
        }
        if all_day != old_event.all_day {
            update_data.all_day = Some(all_day);
            update_count += 1;
        }

        // There is no direct way in Huly to change event recurrency
        // ReccuringEvent is a different object class and must be recreated
        let is_old_recurrent = old_event.rules.is_some();
        if let Some(prop) = ical_event.event.get_property("RRULE") {
            if let Some(value) = &prop.value {
                if !is_old_recurrent {
                    return Err(Error::InvalidData("Unable change event recurrency".into()));
                }
                let rules = parse_rrule_string(value.as_str())?;
                let old_rules = old_event.rules.unwrap();
                if rules != old_rules {
                    update_data.rules = Some(rules);
                    update_count += 1;
                }
            } else if is_old_recurrent {
                return Err(Error::InvalidData("Unable change event recurrency".into()));
            }
        } else if is_old_recurrent {
            return Err(Error::InvalidData("Unable change event recurrency".into()));
        }
        if is_old_recurrent {
            let exdate = self.get_exdate(ical_event)?;
            if old_event.exdate != exdate {
                update_data.exdate = exdate;
                update_count += 1;
            }
        }

        let time_zone = self.get_timezone(ical_event)?;
        if let Some(time_zone) = time_zone {
            if let Some(old_time_zone) = old_event.time_zone {
                if time_zone != old_time_zone {
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

        let tx_id = generate_id(cache.api_url(), user.try_into()?,"core:class:TxUpdateDoc").await?;

        let update_tx = HulyEventTx {
            id: tx_id,
            class: "core:class:TxUpdateDoc".to_string(),
            space: "core:space:Tx".to_string(),
            // TODO: extract user ids from workspace token
            modified_by: old_event.modified_by.clone(),
            created_by: old_event.modified_by.clone(),
            object_id: old_event.id.clone(),
            object_class: old_event.class.clone(),
            object_space: old_event.space.clone(),
            operations: Some(update_data),
            attributes: None,
            collection: old_event.collection.clone(),
            attached_to: old_event.attached_to.clone(),
            attached_to_class: old_event.attached_to_class.clone(),
        };

        println!("*** UPDATE_TX {}", serde_json::to_string_pretty(&update_tx).unwrap());
        tx(cache.api_url(), user.try_into()?, update_tx).await?;

        cache.invalidate(user);
        Ok(())
    }

    async fn create_event(&self, user: &User, event_id: &str, ical_event: &EventObject) -> Result<(), Error> {
        let mut cache = self.calendar_cache.lock().await;
        let cal_id = cache.get_calendar_id(user).await?;

        let (start, end, all_day) = self.get_timestamps(&ical_event.event)?;
        if start.is_none() {
            return Err(Error::InvalidData("Missing value: DTSTART".into()));
        }
        let start = start.unwrap();
        if end.is_none() {
            return Err(Error::InvalidData("Missing value: DTEND".into()));
        }
        let end = end.unwrap();

        let mut rules = None;
        if let Some(prop) = ical_event.event.get_property("RRULE") {
            if let Some(value) = &prop.value {
                rules = Some(parse_rrule_string(value.as_str())?);
            }
        }

        let create_data = HulyEventCreateData {
            calendar: cal_id.clone(),
            event_id: event_id.to_string(),
            date: start.clone(),
            due_date: end,
            // TODO: handle markdown
            description: "".to_string(),
            // TODO: handle participants
            participants: None,
            // TODO: handle reminders
            reminders: None,
            title: if let Some(prop) = ical_event.event.get_property("SUMMARY") {
                if let Some(value) = &prop.value {
                    value.clone()
                } else {
                    return Err(Error::InvalidData("Missing value: SUMMARY".into()));
                }
            } else {
                return Err(Error::InvalidData("Missing value: SUMMARY".into()));
            },
            location: if let Some(prop) = ical_event.event.get_property("LOCATION") {
                if let Some(value) = &prop.value {
                    Some(value.clone())
                } else {
                    None
                }
            } else {
                None
            },
            all_day,
            // TODO: handle attachments
            time_zone: self.get_timezone(ical_event)?,
            access: "owner".to_string(),
            original_start_time: rules.is_some().then(|| start),
            rules,
            exdate: self.get_exdate(ical_event)?,
        };

        let tx_id = generate_id(cache.api_url(), user.try_into()?,"core:class:TxUpdateDoc").await?;
        let obj_id = generate_id(cache.api_url(), user.try_into()?,"calendar:class:Event").await?;

        let create_tx = HulyEventTx {
            id: tx_id,
            class: "core:class:TxCreateDoc".to_string(),
            space: "core:space:Tx".to_string(),
            // TODO: extract user ids from workspace token
            modified_by: "67850773c1fb9ed8f44589f5".to_string(),
            created_by: "67850773c1fb9ed8f44589f5".to_string(),
            object_id: obj_id,
            object_class: if create_data.rules.is_some() { 
                "calendar:class:ReccuringEvent"
            } else { 
                "calendar:class:Event" 
            }.to_string(),
            object_space: "calendar:space:Calendar".to_string(),
            operations: None,
            attributes: Some(create_data),
            collection: "events".to_string(),
            attached_to: "calendar:ids:NoAttached".to_string(),
            attached_to_class: "calendar:class:Event".to_string(),
        };

        println!("*** CREATE_TX {}", serde_json::to_string_pretty(&create_tx).unwrap());
        tx(cache.api_url(), user.try_into()?, create_tx).await?;

        cache.invalidate(user);
        Ok(())
    }
}
