use rustical_store::Error;
use chrono::{DateTime, Duration, TimeZone, Utc, Offset, Datelike};
use chrono_tz::Tz;
use ical::{
    generator::{IcalCalendar, IcalEvent, Emitter},
    parser::ical::component::{IcalTimeZone, IcalTimeZoneTransition, IcalTimeZoneTransitionType},
    property::Property,
};
use rustical_store::calendar::{CalendarObject, CalendarObjectComponent, EventObject};
use std::collections::HashMap;
use crate::api::{HulyEventData, RecurringRule, Timestamp};
use sha2::{Sha256, Digest};

pub(crate) fn calc_etag(id: &String, modified_on: Timestamp) -> String {
    let mut hasher = Sha256::new();
    hasher.update(id);
    hasher.update(modified_on.to_string());
    format!("{:x}", hasher.finalize())
}

fn timestamp_to_utc(msec: Timestamp, name_hint: &str) -> Result<chrono::DateTime<chrono::Utc>, Error> {
    let secs = msec / 1000;
    let nsecs = ((msec - secs * 1000) * 1000) as u32;
    let dt = chrono::Utc.timestamp_opt(secs, nsecs);
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

pub(crate) fn parse_to_utc_msec(time_str: &str, tz: &chrono_tz::Tz, name_hint: &str) -> Result<i64, Error> {
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

fn get_timezone_transitions(tz: &Tz, year: i32) -> Vec<(DateTime<Utc>, Duration)> {
    let start = tz.with_ymd_and_hms(year, 1, 1, 0, 0, 0).unwrap();
    let end = tz.with_ymd_and_hms(year + 1, 1, 1, 0, 0, 0).unwrap();

    let mut transitions = Vec::new();
    let mut current = start;

    while current < end {
        let next_local = current + Duration::hours(1);
        if next_local >= end { break; }

        let next_offset = tz.offset_from_utc_datetime(&next_local.naive_utc());
        let utc = next_local.naive_utc();

        transitions.push((
            Utc.from_utc_datetime(&utc),
            Duration::seconds(i64::from(next_offset.fix().local_minus_utc()))
        ));

        current = next_local;
    }

    transitions
}

fn add_timezone_transitions(ical_tz: &mut IcalTimeZone, _tz: &Tz, transitions: &[(DateTime<Utc>, Duration)]) {
    if transitions.is_empty() {
        return;
    }

    let mut prev_offset = transitions[0].1;

    for (dt, offset) in transitions {
        let is_dst = offset.num_seconds() > prev_offset.num_seconds();
        let offset_secs = offset.num_seconds() as i32;

        // Create transition component with correct type
        let mut transition = IcalTimeZoneTransition {
            transition: if is_dst {
                IcalTimeZoneTransitionType::DAYLIGHT
            } else {
                IcalTimeZoneTransitionType::STANDARD
            },
            properties: Vec::new(),
        };

        // Add transition time property
        transition.properties.push(Property {
            name: "DTSTART".to_string(),
            params: None,
            value: Some(dt.format("%Y%m%dT%H%M%SZ").to_string()),
        });

        // Add offset property
        transition.properties.push(Property {
            name: "TZOFFSETFROM".to_string(),
            params: None,
            value: Some(format!("{:+05}", prev_offset.num_seconds() / 3600)),
        });

        transition.properties.push(Property {
            name: "TZOFFSETTO".to_string(),
            params: None,
            value: Some(format!("{:+05}", offset_secs / 3600)),
        });

        ical_tz.transitions.push(transition);
        prev_offset = *offset;
    }
}

impl TryInto<CalendarObject> for HulyEventData {
    type Error = Error;

    fn try_into(self) -> Result<CalendarObject, Self::Error> {
        let time_zone = self.time_zone.unwrap_or_else(|| "UTC".to_string());
        let tz: Tz = time_zone.parse().map_err(|_| Error::InvalidData("Invalid timezone".to_string()))?;

        // Create timezone component if not UTC
        let mut ical_tz = IcalTimeZone {
            transitions: Vec::new(),
            properties: Vec::new(),
        };

        // Add TZID property
        ical_tz.properties.push(Property {
            name: "TZID".to_string(),
            params: None,
            value: Some(time_zone.clone()),
        });

        // Create calendar object with VERSION and PRODID
        let mut ical_cal = IcalCalendar::new();
        ical_cal.properties.push(Property {
            name: "TZID".to_string(),
            params: None,
            value: Some(time_zone.clone()),
        });

        // Create event component
        let mut ical_event = IcalEvent::new();
        ical_event.properties.push(Property {
            name: "UID".to_string(),
            params: None,
            value: Some(self.id.clone()),
        });
        ical_event.properties.push(Property {
            name: "SUMMARY".to_string(),
            params: None,
            value: Some(self.title),
        });

        // Add description
        ical_event.properties.push(Property {
            name: "DESCRIPTION".to_string(),
            params: None,
            value: Some(self.description),
        });

        // Add start and end dates
        let start_dt = timestamp_to_utc(self.date, "start date")?;
        let end_dt = timestamp_to_utc(self.due_date, "due date")?;
        let start_year = start_dt.year();
        let end_year = end_dt.year();

        // Get timezone transitions for the relevant years
        for year in start_year..=end_year {
            let transitions = get_timezone_transitions(&tz, year);
            if !transitions.is_empty() {
                add_timezone_transitions(&mut ical_tz, &tz, &transitions);
            }
        }

        // Add start and end dates to event
        ical_event.properties.push(Property {
            name: "DTSTART".to_string(),
            params: Some(vec![("TZID".to_string(), vec![time_zone.clone()])]),
            value: Some(format_utc_msec(self.date, &tz, false, "start date")?),
        });

        ical_event.properties.push(Property {
            name: "DTEND".to_string(),
            params: Some(vec![("TZID".to_string(), vec![time_zone.clone()])]),
            value: Some(format_utc_msec(self.due_date, &tz, false, "due date")?),
        });

        // Add VERSION and PRODID properties
        ical_cal.properties.push(Property {
            name: "VERSION".to_string(),
            params: None,
            value: Some("2.0".to_string()),
        });
        ical_cal.properties.push(Property {
            name: "PRODID".to_string(),
            params: None,
            value: Some("-//Huly Labs//NONSGML Huly Calendar//EN".to_string()),
        });

        // Add timezone and event components to calendar
        if time_zone != "UTC" {
            ical_cal.timezones.push(ical_tz.clone());
        }
        ical_cal.events.push(ical_event.clone());

        // Create calendar object
        let mut timezones = HashMap::new();
        if time_zone != "UTC" {
            timezones.insert(time_zone, ical_tz);
        }

        Ok(CalendarObject {
            id: self.id,
            ics: ical_cal.generate(),
            etag: None,
            data: CalendarObjectComponent::Event(EventObject {
                event: ical_event,
                timezones,
            }),
        })
    }
}

pub(crate) fn parse_rrule_string(rrules: &str) -> Result<Vec<RecurringRule>, Error> {
    let rules = rrules.split('\n')
        .filter(|s| !s.is_empty())
        .map(RecurringRule::from_rrule_string)
        .collect();
    if let Err(err) = rules {
        return Err(Error::InvalidData(format!("Invalid RRULE: {}", err)));
    }
    Ok(rules.unwrap())
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

        // Add remaining RRULE parts
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
