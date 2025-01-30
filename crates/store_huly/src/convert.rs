use std::collections::HashMap;
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, Offset, TimeZone, Utc};
use chrono_tz::OffsetComponents;
use rustical_store::{CalendarObject, Error};
use ical::generator::{Emitter, IcalCalendarBuilder, IcalEvent, IcalEventBuilder};
use ical::parser::Component;
use ical::parser::ical::component::{IcalTimeZone, IcalTimeZoneTransition, IcalTimeZoneTransitionType};
use ical::property::Property;
use rustical_store::calendar::{CalendarObjectComponent, EventObject};
use sha2::{Digest, Sha256};
use std::str::FromStr;
use crate::api::{HulyEvent, HulyEventData, HulyEventCreateData, RecurringRule, Timestamp};

pub(crate) fn calc_etag(id: &String, modified_on: Timestamp) -> String {
    let mut hasher = Sha256::new();
    hasher.update(id);
    hasher.update(modified_on.to_string());
    format!("{:x}", hasher.finalize())
}

fn timestamp_to_utc(msec: Timestamp, name_hint: &str) -> Result<DateTime<Utc>, Error> {
    let secs = msec / 1000;
    let nsecs = ((msec - secs * 1000) * 1000) as u32;
    let dt = Utc.timestamp_opt(secs, nsecs);
    let chrono::offset::LocalResult::Single(utc) = dt else {
        return Err(Error::InvalidData(format!("Invalid timestamp: {}", name_hint)))
    };
    Ok(utc)
}

fn format_utc_msec(msec: Timestamp, tz: &chrono_tz::Tz, all_day: bool, name_hint: &str) -> Result<String, Error> {
    let utc = timestamp_to_utc(msec, name_hint)?;
    if all_day {
        return Ok(utc.format("%Y%m%d").to_string());
    }
    let tz_aware = utc.with_timezone(tz);
    Ok(tz_aware.format(ical::generator::ICAL_DATE_FORMAT).to_string())
}

fn parse_to_utc_msec(time_str: &str, tz: &chrono_tz::Tz, name_hint: &str) -> Result<i64, Error> {
    let local = chrono::NaiveDateTime::parse_from_str(time_str, ical::generator::ICAL_DATE_FORMAT);
    if let Err(err) = local {
        return Err(Error::InvalidData(format!("Invalid timestamp: {}: {}", name_hint, err)))
    }
    let local = local.unwrap();
    let Some(tz_aware) = tz.from_local_datetime(&local).earliest() else {
        return Err(Error::InvalidData(format!("Invalid timestamp: {}", name_hint)))
    };
    Ok(tz_aware.timestamp_millis())
}

struct TimezoneTransition {
    utc: NaiveDateTime,
    offset: Duration,
    is_dst: bool,
}

fn get_timezone_transitions(tz: &chrono_tz::Tz, year: i32) -> Vec<TimezoneTransition> {
    let start = Utc.with_ymd_and_hms(year, 1, 1, 0, 0, 0).unwrap();
    let end = Utc.with_ymd_and_hms(year + 1, 1, 1, 0, 0, 0).unwrap();
    let mut transitions = Vec::new();
    let mut curr = start;
    let mut prev_offset_secs = None;
    while curr < end {
        let utc = curr.naive_utc();
        let offset = tz.offset_from_utc_datetime(&utc);
        let offset_secs = i64::from(offset.fix().local_minus_utc());
        if let Some(prev_offset) = prev_offset_secs {
            if offset_secs != prev_offset {
                transitions.push(TimezoneTransition{
                    utc,
                    offset: Duration::seconds(offset_secs),
                    is_dst: !offset.dst_offset().is_zero(),
                });
                prev_offset_secs = Some(offset_secs);
            }
        } else {
            prev_offset_secs = Some(offset_secs);
        }
        curr += Duration::hours(1);
    }
    transitions
}

fn add_timezone_transitions(ical_tz: &mut IcalTimeZone, transitions: &[TimezoneTransition]) {
    let mut prev = &transitions[0];
    for curr in transitions {
        let mut transition = IcalTimeZoneTransition {
            transition: if curr.is_dst {
                IcalTimeZoneTransitionType::DAYLIGHT
            } else {
                IcalTimeZoneTransitionType::STANDARD
            },
            properties: Vec::new(),
        };
        transition.properties.push(Property {
            name: "DTSTART".to_string(),
            params: None,
            value: Some(curr.utc.format("%Y%m%dT%H%M%SZ").to_string()),
        });
        transition.properties.push(Property {
            name: "TZOFFSETFROM".to_string(),
            params: None,
            value: Some(format!("{:+05}", prev.offset.num_seconds() / 3600)),
        });
        transition.properties.push(Property {
            name: "TZOFFSETTO".to_string(),
            params: None,
            value: Some(format!("{:+05}", curr.offset.num_seconds() / 3600)),
        });
        ical_tz.transitions.push(transition);
        prev = curr;
    }
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

pub(crate) fn parse_rrule_string(rrules: &str) -> Result<Option<Vec<RecurringRule>>, Error> {
    let rules = rrules.split('\n')
        .filter(|s| !s.is_empty())
        .map(RecurringRule::from_rrule_string)
        .collect::<Result<Vec<_>, _>>();
    if let Err(err) = rules {
        return Err(Error::InvalidData(format!("Invalid RRULE: {}", err)));
    }
    let rules = rules.unwrap();
    Ok(if rules.is_empty() {
        None
    } else {
        Some(rules)
    })
}

impl RecurringRule {
    fn to_rrule_string(&self) -> Result<String, Error> {
        let mut parts = vec![format!("FREQ={}", self.freq.to_uppercase())];
        if let Some(end_date) = &self.end_date {
            let end_date = timestamp_to_utc(*end_date, "rrule.enddate")?;
            let end_date = end_date.format("%Y%m%dT%H%M%SZ").to_string();
            parts.push(format!("UNTIL={}", end_date));
        }
        if let Some(count) = self.count {
            parts.push(format!("COUNT={}", count));
        }
        if let Some(interval) = self.interval {
            parts.push(format!("INTERVAL={}", interval));
        }
        if let Some(by_second) = &self.by_second {
            parts.push(format!("BYSECOND={}", by_second.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",")));
        }
        if let Some(by_minute) = &self.by_minute {
            parts.push(format!("BYMINUTE={}", by_minute.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",")));
        }
        if let Some(by_hour) = &self.by_hour {
            parts.push(format!("BYHOUR={}", by_hour.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",")));
        }
        if let Some(by_day) = &self.by_day {
            parts.push(format!("BYDAY={}", by_day.join(",")));
        }
        if let Some(by_month_day) = &self.by_month_day {
            parts.push(format!("BYMONTHDAY={}", by_month_day.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",")));
        }
        if let Some(by_year_day) = &self.by_year_day {
            parts.push(format!("BYYEARDAY={}", by_year_day.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",")));
        }
        if let Some(by_week_no) = &self.by_week_no {
            parts.push(format!("BYWEEKNO={}", by_week_no.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",")));
        }
        if let Some(by_month) = &self.by_month {
            parts.push(format!("BYMONTH={}", by_month.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",")));
        }
        if let Some(by_set_pos) = &self.by_set_pos {
            parts.push(format!("BYSETPOS={}", by_set_pos.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",")));
        }
        if let Some(wkst) = &self.wkst {
            parts.push(format!("WKST={}", wkst.join(",")));
        }
        Ok(parts.join(";"))
    }

    pub(crate) fn from_rrule_string(rrule: &str) -> Result<Self, String> {
        let mut rule = RecurringRule::default();
        for part in rrule.split(';') {
            let mut kv = part.split('=');
            let key = kv.next().ok_or("Invalid format")?;
            let value = kv.next().ok_or("Invalid format")?;
            match key.to_uppercase().as_str() {
                "FREQ" => rule.freq = value.to_string(),
                "UNTIL" => {
                    let dt = chrono::NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%SZ");
                    if let Err(err) = dt {
                        return Err(format!("Invalid date UNTIL: {}", err))
                    }
                    rule.end_date = Some(dt.unwrap().and_utc().timestamp_millis())
                }
                "COUNT" => {
                    rule.count = Some(value.parse().map_err(|e| format!("Invalid COUNT: {}", e))?);
                }
                "INTERVAL" => {
                    rule.interval = Some(value.parse().map_err(|e| format!("Invalid INTERVAL: {}", e))?);
                }
                "BYSECOND" => {
                    rule.by_second = Some(value.split(',')
                        .map(|s| s.parse::<u8>())
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| format!("Invalid BYSECOND: {}", e))?);
                }
                "BYMINUTE" => {
                    rule.by_minute = Some(value.split(',')
                        .map(|s| s.parse::<u8>())
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| format!("Invalid BYMINUTE: {}", e))?);
                }
                "BYHOUR" => {
                    rule.by_hour = Some(value.split(',')
                        .map(|s| s.parse::<u8>())
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| format!("Invalid BYHOUR: {}", e))?);
                }
                "BYDAY" => {
                    rule.by_day = Some(value.split(',')
                        .map(|s| s.to_string())
                        .collect());
                }
                "BYMONTHDAY" => {
                    rule.by_month_day = Some(value.split(',')
                        .map(|s| s.parse::<u8>())
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| format!("Invalid BYMONTHDAY: {}", e))?);
                }
                "BYYEARDAY" => {
                    rule.by_year_day = Some(value.split(',')
                        .map(|s| s.parse::<u16>())
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| format!("Invalid BYYEARDAY: {}", e))?);
                }
                "BYWEEKNO" => {
                    rule.by_week_no = Some(value.split(',')
                        .map(|s| s.parse::<i8>())
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| format!("Invalid BYWEEKNO: {}", e))?);
                }
                "BYMONTH" => {
                    rule.by_month = Some(value.split(',')
                        .map(|s| s.parse::<u8>())
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| format!("Invalid BYMONTH: {}", e))?);
                }
                "BYSETPOS" => {
                    rule.by_set_pos = Some(value.split(',')
                        .map(|s| s.parse::<i16>())
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|e| format!("Invalid BYSETPOS: {}", e))?);
                }
                "WKST" => {
                    rule.wkst = Some(value.split(',')
                        .map(|s| s.to_string())
                        .collect());
                }
                _ => return Err(format!("Unknown part: {}", key))
            }
        }
        if rule.freq.is_empty() {
            return Err("FREQ is required".to_string());
        }
        Ok(rule)
    }
}

pub fn from_ical_get_timestamp(prop: &ical::property::Property, prop_hint: &str) -> Result<(Option<Timestamp>, bool), Error> {
    let Some(value) = &prop.value else {
        return Err(Error::InvalidData(format!("Missing value: {}", prop_hint)));
    };
    let Some(params) = &prop.params else {
        let utc = NaiveDateTime::parse_from_str(value.as_str(), "%Y%m%dT%H%M%SZ");
        if let Err(err) = utc {
            return Err(Error::InvalidData(format!("invalid utc date: {}: {}", prop_hint, err)));
        }
        let utc = utc.unwrap();
        let ms = utc.and_utc().timestamp_millis();
        return Ok((Some(ms), false));
    };
    for (param_name, param_values) in params {
        match param_name.as_str() {
            // params=Some([("VALUE", ["DATE"])]),
            "VALUE" => {
                if param_values.contains(&"DATE".to_string()) {
                    let local = NaiveDate::parse_from_str(value.as_str(), "%Y%m%d");
                    if let Err(err) = local {
                        return Err(Error::InvalidData(format!("invalid date: {}: {}", prop_hint, err)));
                    }
                    let local = local.unwrap();
                    let Some(dt) = local.and_hms_opt(0, 0, 0) else {
                        return Err(Error::InvalidData(format!("invalid date-time: {}", prop_hint)));
                    };
                    let ms = dt.and_utc().timestamp_millis();
                    return Ok((Some(ms), true));
                }
            },
            // params=Some([("TZID", ["Asia/Novosibirsk"])]),
            "TZID" => {
                if param_values.is_empty() {
                    return Err(Error::InvalidData(format!("timezone not set: {}", prop_hint)));
                }
                let tzid = param_values[0].as_str();
                let tz = chrono_tz::Tz::from_str(tzid);
                if let Err(err) = tz {
                    return Err(Error::InvalidData(format!("invalid timezone: {}: {}", prop_hint, err)));
                }
                let tz = tz.unwrap();
                let ms = parse_to_utc_msec(value.as_str(), &tz, prop_hint)?;
                return Ok((Some(ms), false));
            },
            _ => {},
        }
    }
    return Ok((None, false))
}

pub(crate) fn from_ical_get_timestamps(event: &IcalEvent) -> Result<(Option<Timestamp>, Option<Timestamp>, bool), Error> {
    let (start, all_day_1) = if let Some(prop) = event.get_property("DTSTART") {
        from_ical_get_timestamp(prop, "DTSTART")?
    } else {
        (None, false)
    };
    let (end, all_day_2) = if let Some(prop) = event.get_property("DTEND") {
        from_ical_get_timestamp(prop, "DTEND")?
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

pub(crate) fn from_ical_get_timestamp_required(event: &IcalEvent, prop_name: &str) -> Result<Timestamp, Error> {
    let prop = event.get_property(prop_name)
        .ok_or_else(|| Error::InvalidData(format!("Missing prop: {}", prop_name)))?;
    let (ts, _) = from_ical_get_timestamp(prop, prop_name)?;
    ts.ok_or_else(|| Error::InvalidData(format!("Missing field value: {}", prop_name)))
}

pub(crate) fn from_ical_get_exdate(ical_event: &EventObject) -> Result<Option<Vec<Timestamp>>, Error> {
    let mut exdate = Vec::new();
        for prop in &ical_event.event.properties {
            if prop.name == "EXDATE" {
                let (dt, _) = from_ical_get_timestamp(prop, "EXDATE")?;
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

pub(crate) fn from_ical_get_timezone(ical_event: &EventObject) -> Result<Option<String>, Error> {
    if ical_event.timezones.len() > 1 {
        return Err(Error::InvalidData("multiple timezones not supported".into()))
    }
    let tzids: Vec<String> = ical_event.timezones.keys().cloned().collect();
    Ok(tzids.first().cloned())
}

impl HulyEventCreateData {
    pub(crate) fn new(cal_id: &str, event_id: &str, event_obj: &EventObject) -> Result<Self, Error> {
        let (start, end, all_day) = from_ical_get_timestamps(&event_obj.event)?;
        if start.is_none() {
            return Err(Error::InvalidData("Missing value: DTSTART".into()));
        }
        let start = start.unwrap();
        if end.is_none() {
            return Err(Error::InvalidData("Missing value: DTEND".into()));
        }
        let end = end.unwrap();

        let mut rules = None;
        if let Some(prop) = event_obj.event.get_property("RRULE") {
            if let Some(value) = &prop.value {
                rules = parse_rrule_string(value.as_str())?;
            }
        }

        Ok(Self {
            calendar: cal_id.to_string(),
            event_id: event_id.to_string(),
            date: start.clone(),
            due_date: end,
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
            original_start_time: rules.is_some().then(|| start),
            rules,
            exdate: from_ical_get_exdate(event_obj)?,
            recurring_event_id: None,
        })
    }
}
