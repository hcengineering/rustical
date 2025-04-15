use crate::api::{HulyEvent, HulyEventCreateData, HulyEventData, HulyEventUpdateData, Timestamp};
use crate::auth::HulyUser;
use crate::convert_rrule::parse_rrule_string;
use crate::convert_time::{
    format_duration_rfc5545, format_utc_msec, from_ical_get_event_bounds, from_ical_get_exdate,
    from_ical_get_timezone,
};
use ical::generator::{Emitter, IcalCalendarBuilder, IcalEvent, IcalEventBuilder};
use ical::parser::ical::component::{IcalAlarm, IcalTimeZone};
use ical::parser::Component;
use ical::property::Property;
use ical::{ical_param, ical_property};
use rustical_store::calendar::{parse_duration, CalendarObjectComponent, EventObject};
use rustical_store::{CalendarObject, Error};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::str::FromStr;

pub(crate) fn calc_etag(id: &String, modified_on: Timestamp) -> String {
    let mut hasher = Sha256::new();
    hasher.update(id);
    hasher.update(modified_on.to_string());
    format!("{:x}", hasher.finalize())
}

pub(crate) fn from_ical_get_alarms(event: &IcalEvent) -> Result<Option<Vec<Timestamp>>, Error> {
    let mut alarms = Vec::new();
    for ical_alarm in &event.alarms {
        let prop = ical_alarm
            .get_property("TRIGGER")
            .ok_or_else(|| Error::InvalidData("Missing property: TRIGGER".into()))?;
        if let Some(param) = prop.params.as_ref() {
            if param.iter().any(|(name, vals)| {
                if name == "RELATED" {
                    vals.iter().any(|val| val != "START")
                } else {
                    false
                }
            }) {
                return Err(Error::InvalidData(
                    "Only triggers related to the start of the event are supported".into(),
                ));
            }
        }
        let value = prop
            .value
            .as_ref()
            .ok_or_else(|| Error::InvalidData("Missing value: TRIGGER".into()))?;
        let dist = parse_duration(value).map_err(|_| {
            Error::InvalidData("Only triggers in the form of DURATION are supported".into())
        })?;
        if dist > chrono::TimeDelta::zero() {
            return Err(Error::InvalidData(
                "Only triggers that are before the events are supported".into(),
            ));
        }
        alarms.push(-dist.num_milliseconds());
    }
    if alarms.is_empty() {
        return Ok(None);
    }
    Ok(Some(alarms))
}

pub(crate) fn from_ical_get_participants(event: &IcalEvent) -> Result<Option<Vec<String>>, Error> {
    let mut participants = Vec::new();
    for prop in &event.properties {
        if prop.name == "ATTENDEE" {
            if let Some(value) = &prop.value {
                participants.push(value.clone());
            }
        }
    }
    Ok(if participants.is_empty() {
        None
    } else {
        Some(participants)
    })
}

fn make_ical_event(event: &HulyEventData, tz: &chrono_tz::Tz) -> Result<IcalEvent, Error> {
    let created = format_utc_msec(event.created_on, tz, false, "created")?;
    let changed = format_utc_msec(event.modified_on, tz, false, "modified")?;
    let start = format_utc_msec(event.date, tz, event.all_day, "start date")?;
    let mut due_date = event.due_date;
    if event.all_day {
        // Huly defines all-day event as date={start_day}{00:00:00} due_date={end_day}{23:59:59:999}
        // While CaldDav clients expect DTEND={end_day+1}{no time part}
        // Shifting due_date by 1 ms, we switch to the next day
        due_date += 1;
    }
    let end = format_utc_msec(due_date, tz, event.all_day, "due date")?;

    let ical_event = IcalEventBuilder::tzid(tz.name())
        .uid(event.event_id.clone().unwrap())
        .changed(changed.clone());

    let ical_event = if event.all_day {
        ical_event.start_day(start).end_day(end)
    } else {
        ical_event.start(start).end(end)
    };

    let mut ical_event = ical_event
        .set(ical_property!("SUMMARY", &event.title))
        .set(ical_property!(
            "DESCRIPTION",
            extract_text_from_markup(&event.description)
        ))
        .set(ical_property!("CREATED", &created))
        .set(ical_property!("LAST_MODIFIED", &changed));

    if let Some(location) = &event.location {
        ical_event = ical_event.set(ical_property!("LOCATION", location));
    }

    if let Some(rules) = &event.rules {
        if !rules.is_empty() {
            let mut rrules: Vec<String> = vec![];
            for rule in rules {
                let rrule = rule.to_rrule_string()?;
                rrules.push(rrule);
            }
            ical_event = ical_event.repeat_rule(rrules.join(";"));
        }
    }

    if let Some(exdate) = &event.exdate {
        for dt in exdate {
            let dt_str = format_utc_msec(*dt, tz, true, "exdate")?;
            ical_event = ical_event.set(ical_property!(
                "EXDATE",
                dt_str,
                ical_param!("VALUE", "DATE")
            ));
        }
    }

    let mut ical_event = ical_event.build();

    if let Some(reminders) = &event.reminders {
        for reminder in reminders {
            let mut alarm = IcalAlarm::new();
            alarm.properties.push(ical_property!(
                "TRIGGER",
                format_duration_rfc5545(-reminder),
                ical_param!("RELATED", "START")
            ));
            alarm.properties.push(ical_property!("ACTION", "DISPLAY"));
            alarm
                .properties
                .push(ical_property!("DESCRIPTION", event.title.clone()));
            ical_event.alarms.push(alarm);
        }
    }

    // TODO: handle event.participants
    // TODO: add prop ORGANIZER (from user.id?)
    if let Some(participants) = &event.external_participants {
        for participant in participants {
            ical_event.add_property(ical_property!(
                "ATTENDEE",
                participant,
                ical_param!("ROLE", "REQ-PARTICIPANT")
            ));
        }
    }

    Ok(ical_event)
}

impl TryInto<CalendarObject> for HulyEvent {
    type Error = Error;

    fn try_into(self) -> Result<CalendarObject, Self::Error> {
        // let time_zone = self.time_zone.as_ref()
        //     .ok_or(Error::InvalidData("No event time zone".into()))?;
        let utc_time_zone = "UTC".to_string();

        let event = &self.data;
        let mut modified_on = event.modified_on;

        let time_zone = event.time_zone.as_ref().unwrap_or(&utc_time_zone);
        let tz = chrono_tz::Tz::from_str(time_zone)
            .map_err(|err| Error::InvalidData(format!("Invalid event timezone: {}", err)))?;

        let mut ical_event = make_ical_event(event, &tz)?;

        let mut ical_instances = Vec::new();
        if let Some(instances) = &self.instances {
            for instance in instances.iter() {
                if instance.modified_on > modified_on {
                    modified_on = instance.modified_on;
                }

                if instance.is_cancelled.unwrap_or(false) {
                    let dt_str = format_utc_msec(instance.date, &tz, true, "exdate")?;
                    ical_event.add_property(ical_property!(
                        "EXDATE",
                        dt_str,
                        ical_param!("VALUE", "DATE")
                    ));
                    continue;
                }

                let Some(original_start_time) = instance.original_start_time else {
                    return Err(Error::InvalidData(
                        "Missing value: original_start_time".into(),
                    ));
                };

                let mut instance = instance.clone();
                instance.event_id = event.event_id.clone();
                instance.rules = None;
                instance.exdate = None;

                let orig_start = format_utc_msec(
                    original_start_time,
                    &tz,
                    event.all_day,
                    "original_start_time",
                )?;

                let mut ical_instance = make_ical_event(&instance, &tz)?;
                ical_instance.add_property(ical_property!(
                    "RECURRENCE-ID",
                    orig_start,
                    ical_param!("TZID", tz.name())
                ));
                ical_instance.add_property(ical_property!("SEQUENCE", "1"));
                ical_instances.push(ical_instance);
            }
        }

        let mut ical_tz = IcalTimeZone::new();
        ical_tz.add_property(ical_property!("TZID", tz.name()));
        ical_tz.add_property(ical_property!("X-LIC-LOCATION", tz.name()));

        // TODO: The "VTIMEZONE" calendar component MUST include
        // at least one definition of a "STANDARD" or "DAYLIGHT" sub-component
        // https://www.rfc-editor.org/rfc/rfc5545#section-3.6.5
        // This is not a proper solution, because it returns transitions only if it detects a DST change in year
        // But the standard says that there must be at least one transition per year
        /*
        let start_dt = timestamp_to_utc(self.date, "start date")?;
        let end_dt = timestamp_to_utc(self.due_date, "due date")?;
        let start_year = start_dt.year();
        let end_year = end_dt.year();
        for year in start_year..=end_year {
            let transitions = get_timezone_transitions(&tz, year);
            if !transitions.is_empty() {
                add_timezone_transitions(&mut ical_tz, &transitions);
            }
        }
        */

        let mut ical_obj = IcalCalendarBuilder::version("2.0")
            .gregorian()
            .prodid("-//Huly Labs//NONSGML Huly Calendar//EN")
            .add_tz(ical_tz.clone())
            .add_event(ical_event.clone());
        for ical_event in ical_instances.into_iter() {
            ical_obj = ical_obj.add_event(ical_event);
        }
        let ical_obj = ical_obj.build();

        let etag = calc_etag(event.event_id.as_ref().unwrap(), modified_on);

        let obj = CalendarObject {
            id: event.event_id.clone().unwrap(),
            ics: ical_obj.generate(),
            etag: Some(etag),
            data: CalendarObjectComponent::Event(EventObject {
                event: ical_event,
                timezones: HashMap::from([(tz.name().to_string(), ical_tz)]),
            }),
        };

        Ok(obj)
    }
}

impl HulyEventCreateData {
    pub(crate) fn new(
        user: &HulyUser,
        cal_id: &str,
        event_id: &str,
        event_obj: &EventObject,
    ) -> Result<Self, Error> {
        let (date, due_date, all_day) = from_ical_get_event_bounds(&event_obj.event)?;

        let mut rules = None;
        if let Some(prop) = event_obj.event.get_property("RRULE") {
            if let Some(value) = &prop.value {
                rules = parse_rrule_string(value.as_str())?;
            }
        }

        Ok(Self {
            calendar: cal_id.to_string(),
            event_id: event_id.to_string(),
            date,
            due_date,
            description: if let Some(prop) = event_obj.event.get_property("DESCRIPTION") {
                if let Some(value) = &prop.value {
                    value.clone()
                } else {
                    "".to_string()
                }
            } else {
                "".to_string()
            },
            participants: Some(vec![user.contact_id.clone()]),
            // TODO: make global and local persons from ical participants and add then to event.participants
            external_participants: from_ical_get_participants(&event_obj.event)?,
            reminders: from_ical_get_alarms(&event_obj.event)?,
            title: if let Some(prop) = event_obj.event.get_property("SUMMARY") {
                if let Some(value) = &prop.value {
                    value.clone()
                } else {
                    "".to_string()
                }
            } else {
                "".to_string()
            },
            location: if let Some(prop) = event_obj.event.get_property("LOCATION") {
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
            time_zone: from_ical_get_timezone(event_obj)?,
            access: "owner".to_string(),
            original_start_time: rules.is_some().then(|| date),
            rules,
            exdate: from_ical_get_exdate(&event_obj.event)?,
            recurring_event_id: None,
        })
    }
}

impl HulyEventUpdateData {
    pub(crate) fn new(
        old_event: &HulyEventData,
        event_obj: &EventObject,
    ) -> Result<Option<Self>, Error> {
        let mut update_data = HulyEventUpdateData::default();
        //let mut update_count = 0;
        let mut updates = vec![];
        if let Some(prop) = event_obj.event.get_property("SUMMARY") {
            if let Some(value) = &prop.value {
                if *value != old_event.title {
                    update_data.title = Some(value.clone());
                    updates.push("title");
                    //update_count += 1;
                }
            }
        }
        if let Some(prop) = event_obj.event.get_property("DESCRIPTION") {
            if let Some(value) = &prop.value {
                let old_value = extract_text_from_markup(&old_event.description);
                if *value != old_value {
                    update_data.description = Some(value.clone());
                    updates.push("description");
                    //update_count += 1;
                }
            }
        }
        if let Some(prop) = event_obj.event.get_property("LOCATION") {
            if let Some(value) = &prop.value {
                if let Some(old_value) = &old_event.location {
                    if value != old_value {
                        update_data.location = Some(value.to_string());
                        updates.push("location");
                        //update_count += 1;
                    }
                } else {
                    update_data.location = Some(value.to_string());
                    updates.push("location");
                    //update_count += 1;
                }
            }
        }
        let (date, due_date, all_day) = from_ical_get_event_bounds(&event_obj.event)?;
        if date != old_event.date {
            update_data.date = Some(date);
            updates.push("date");
            //update_count += 1;
        }
        if due_date != old_event.due_date {
            update_data.due_date = Some(due_date);
            updates.push("due_date");
            //update_count += 1;
        }
        if all_day != old_event.all_day {
            update_data.all_day = Some(all_day);
            updates.push("all_day");
            //update_count += 1;
        }
        let reminders = from_ical_get_alarms(&event_obj.event)?;
        if reminders != old_event.reminders {
            update_data.reminders = reminders;
            updates.push("reminders");
            //update_count += 1;
        }

        // There is no direct way in Huly to change event recurrency
        // ReccuringEvent is a different object class and must be recreated
        let is_old_recurrent = old_event
            .rules
            .as_ref()
            .and_then(|r| Some(!r.is_empty()))
            .unwrap_or(false);
        if let Some(prop) = event_obj.event.get_property("RRULE") {
            if let Some(value) = &prop.value {
                if !is_old_recurrent {
                    return Err(Error::InvalidData("Unable change event recurrency".into()));
                }
                let rules = parse_rrule_string(value.as_str())?;
                if old_event.rules != rules {
                    update_data.rules = rules;
                    updates.push("rules");
                    //update_count += 1;
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
                updates.push("exdate");
                //update_count += 1;
            }
        }

        let time_zone = from_ical_get_timezone(event_obj)?;
        if let Some(time_zone) = time_zone {
            if let Some(old_time_zone) = &old_event.time_zone {
                if &time_zone != old_time_zone {
                    update_data.time_zone = Some(time_zone);
                    updates.push("time_zone");
                    //update_count += 1;
                }
            } else {
                update_data.time_zone = Some(time_zone);
                updates.push("time_zone");
                //update_count += 1;
            }
        } else if old_event.time_zone.is_some() {
            update_data.time_zone = None;
            updates.push("time_zone");
            //update_count += 1;
        }

        let participants = from_ical_get_participants(&event_obj.event)?;
        if participants != old_event.external_participants {
            update_data.external_participants = participants;
            updates.push("external_participants");
            //update_count += 1;
        }

        // if update_count == 0 {
        //     return Ok(None);
        // }
        if updates.is_empty() {
            return Ok(None);
        }
        println!("#### UPDATED_PROPS: {:?}", updates);
        Ok(Some(update_data))
    }
}

fn process_json_value(json: &serde_json::Value, result: &mut String) {
    if let Some(obj) = json.as_object() {
        for key in obj.keys() {
            if let Some(v) = obj.get(key) {
                if key == "text" {
                    if let Some(text) = v.as_str() {
                        result.push_str(text);
                        result.push_str(" ");
                    }
                } else if v.is_object() {
                    process_json_value(v, result);
                } else if v.is_array() {
                    for item in v.as_array().unwrap() {
                        process_json_value(item, result);
                    }
                }
            }
        }
    }
}

fn extract_text_from_markup(markup: &str) -> String {
    let json: Result<serde_json::Value, serde_json::Error> = serde_json::from_str(markup);
    if let Err(_) = json {
        return markup.to_string();
    }
    let mut result = String::new();
    let json = json.unwrap();
    process_json_value(&json, &mut result);
    result
}

#[test]
fn test_extract_text_from_markup() {
    let s = r#"{
        "type":"doc",
        "content":[
            {
                "type":"paragraph",
                "content":[
                    {
                        "type":"text",
                        "text":"Hello"
                    }
                ]
            }
        ]
    }"#;
    assert_eq!(extract_text_from_markup(s), "Hello ");

    let s = "Hello";
    assert_eq!(extract_text_from_markup(s), "Hello");
}
