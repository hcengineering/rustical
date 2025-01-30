use std::collections::HashMap;
use rustical_store::{CalendarObject, Error};
use ical::generator::{Emitter, IcalCalendarBuilder, IcalEvent, IcalEventBuilder};
use ical::parser::Component;
use ical::parser::ical::component::IcalTimeZone;
use ical::property::Property;
use rustical_store::calendar::{CalendarObjectComponent, EventObject};
use sha2::{Digest, Sha256};
use std::str::FromStr;
use crate::api::{HulyEvent, HulyEventData, HulyEventCreateData, Timestamp};
use crate::convert_rrule::parse_rrule_string;
use crate::convert_time::{format_utc_msec, from_ical_get_event_bounds, from_ical_get_exdate, from_ical_get_timezone};

pub(crate) fn calc_etag(id: &String, modified_on: Timestamp) -> String {
    let mut hasher = Sha256::new();
    hasher.update(id);
    hasher.update(modified_on.to_string());
    format!("{:x}", hasher.finalize())
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

    let ical_event = ical::generator::IcalEventBuilder::tzid(tz.name())
        .uid(event.event_id.clone().unwrap())
        .changed(changed.clone());
    let ical_event = if event.all_day {
        ical_event
            .start_day(start)
            .end_day(end)
    } else {
        ical_event
            .start(start)
            .end(end)
    };
    let mut ical_event = ical_event
        .set(ical::ical_property!("SUMMARY", &event.title))
        .set(ical::ical_property!("DESCRIPTION", &event.description))
        .set(ical::ical_property!("CREATED", &created))
        .set(ical::ical_property!("LAST_MODIFIED", &changed));
    if let Some(location) = &event.location {
        ical_event = ical_event.set(ical::ical_property!("LOCATION", location));
    }
    if let Some(rules) = &event.rules {
        let mut rrules: Vec<String> = vec![];
        for rule in rules {
            let rrule = rule.to_rrule_string()?;
            rrules.push(rrule);
        }
        ical_event = ical_event.repeat_rule(rrules.join(";"));
    }
    if let Some(exdate) = &event.exdate {
        for dt in exdate {
            let dt_str = format_utc_msec(*dt, tz, true, "exdate")?;
            ical_event = ical_event.set(ical::ical_property!("EXDATE", dt_str, ical::ical_param!("VALUE", "DATE")));
        }
    }
    let ical_event = ical_event
        .build();
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
                    ical_event.add_property(ical::ical_property!("EXDATE", dt_str, ical::ical_param!("VALUE", "DATE")));
                    continue;
                }
    
                let Some(original_start_time) = instance.original_start_time else {
                    return Err(Error::InvalidData("Missing value: original_start_time".into()))
                };

                let mut instance = instance.clone();
                instance.event_id = event.event_id.clone();
                instance.rules = None;
                instance.exdate = None;

                let orig_start = format_utc_msec(original_start_time, &tz, event.all_day, "original_start_time")?;

                let mut ical_instance = make_ical_event(&instance, &tz)?;
                ical_instance.add_property(ical::ical_property!("RECURRENCE-ID", orig_start, ical::ical_param!("TZID", tz.name())));
                ical_instance.add_property(ical::ical_property!("SEQUENCE", "1"));
                ical_instances.push(ical_instance);
            }
        }

        let mut ical_tz = IcalTimeZone::new();
            ical_tz.add_property(ical::ical_property!("TZID", tz.name()));
            ical_tz.add_property(ical::ical_property!("X-LIC-LOCATION", tz.name()));

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

        let mut ical_obj = ical::generator::IcalCalendarBuilder::version("2.0")
            .gregorian()
            .prodid("-//Huly Labs//NONSGML Huly Calendar//EN")
            .add_tz(ical_tz.clone())
            .add_event(ical_event.clone());
        for ical_event in ical_instances.into_iter() {
            ical_obj = ical_obj.add_event(ical_event);
        }
        let ical_obj = ical_obj
            .build();

        let etag = calc_etag(event.event_id.as_ref().unwrap(), modified_on);

        let obj = CalendarObject {
            id: event.event_id.clone().unwrap(),
            ics: ical_obj.generate(),
            etag: Some(etag),
            data: CalendarObjectComponent::Event(EventObject {
                event: ical_event,
                timezones: HashMap::from([(tz.name().to_string(), ical_tz)]),
            })
        };

        Ok(obj)
    }
}

impl TryInto<CalendarObject> for HulyEventData {
    type Error = Error;

    fn try_into(self) -> Result<CalendarObject, Self::Error> {
        // let time_zone = self.time_zone.as_ref()
        //     .ok_or(Error::InvalidData("No event time zone".into()))?;
        let utc_time_zone = "Etc/GMT".to_string();

        let time_zone = self.time_zone.as_ref().unwrap_or(&utc_time_zone);
        let tz = chrono_tz::Tz::from_str(time_zone)
            .map_err(|err| Error::InvalidData(format!("Invalid event timezone: {}", err)))?;

        let created = format_utc_msec(self.created_on, &tz, false, "created")?;
        let changed = format_utc_msec(self.modified_on, &tz, false, "modified")?;
        let start = format_utc_msec(self.date, &tz, self.all_day, "start date")?;
        let mut due_date = self.due_date;
        if self.all_day {
            // Huly defines all-day event as date={start_day}{00:00:00} due_date={end_day}{23:59:59:999}
            // While CaldDav clients expect DTEND={end_day+1}{no time part}
            // Shifting due_date by 1 ms, we switch to the next day
            due_date += 1;
        }
        let end = format_utc_msec(due_date, &tz, self.all_day, "due date")?;

        let ical_event = IcalEventBuilder::tzid(time_zone)
            .uid(self.event_id.clone().unwrap())
            .changed(changed.clone());
        let ical_event = if self.all_day {
            ical_event
                .start_day(start)
                .end_day(end)
        } else {
            ical_event
                .start(start)
                .end(end)
        };
        let mut ical_event = ical_event
            .set(ical::ical_property!("SUMMARY", &self.title))
            .set(ical::ical_property!("DESCRIPTION", &self.description))
            .set(ical::ical_property!("CREATED", &created))
            .set(ical::ical_property!("LAST_MODIFIED", &changed));
        if let Some(location) = &self.location {
            ical_event = ical_event.set(ical::ical_property!("LOCATION", location));
        }
        if let Some(rules) = &self.rules {
            let mut rrules: Vec<String> = vec![];
            for rule in rules {
                let rrule = rule.to_rrule_string()?;
                rrules.push(rrule);
            }
            ical_event = ical_event.repeat_rule(rrules.join(";"));
        }
        if let Some(exdate) = &self.exdate {
            for dt in exdate {
                let dt_str = format_utc_msec(*dt, &tz, true, "exdate")?;
                ical_event = ical_event.set(ical::ical_property!("EXDATE", dt_str, ical::ical_param!("VALUE", "DATE")));
            }
        }
        let ical_event = ical_event
            .build();

        let mut ical_tz = ical::parser::ical::component::IcalTimeZone::new();
            ical_tz.add_property(ical::ical_property!("TZID", tz.name()));
            ical_tz.add_property(ical::ical_property!("X-LIC-LOCATION", tz.name()));

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
        let ical_obj = IcalCalendarBuilder::version("2.0")
            .gregorian()
            .prodid("-//Huly Labs//NONSGML Huly Calendar//EN")
            .add_event(ical_event.clone())
            .add_tz(ical_tz.clone())
            .build();

        let etag = calc_etag(self.event_id.as_ref().unwrap(), self.modified_on);

        let obj = CalendarObject {
            id: self.event_id.clone().unwrap(),
            ics: ical_obj.generate(),
            etag: Some(etag),
            data: CalendarObjectComponent::Event(EventObject {
                event: ical_event,
                timezones: HashMap::from([(tz.name().to_string(), ical_tz)]),
            })
        };

        Ok(obj)
    }
}

impl HulyEventCreateData {
    pub(crate) fn new(cal_id: &str, event_id: &str, event_obj: &EventObject) -> Result<Self, Error> {
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
            // TODO: handle markdown
            description: "".to_string(),
            // TODO: handle participants
            participants: None,
            // TODO: handle reminders
            reminders: None,
            title: if let Some(prop) = event_obj.event.get_property("SUMMARY") {
                if let Some(value) = &prop.value {
                    value.clone()
                } else {
                    return Err(Error::InvalidData("Missing value: SUMMARY".into()));
                }
            } else {
                return Err(Error::InvalidData("Missing value: SUMMARY".into()));
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
