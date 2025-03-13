#![allow(unused_imports)]
use crate::api::Timestamp;
use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use ical::generator::IcalEvent;
use ical::parser::ical::component::IcalTimeZone;
use ical::parser::Component;
use ical::property::Property;
use ical::{ical_param, ical_property};
use rustical_store::calendar::{parse_duration, EventObject};
use rustical_store::Error;
use std::collections::HashMap;
use std::str::FromStr;

pub(crate) fn timestamp_to_utc(msec: Timestamp, name_hint: &str) -> Result<DateTime<Utc>, Error> {
    let secs = msec / 1000;
    let nsecs = ((msec - secs * 1000) * 1000) as u32;
    let dt = Utc.timestamp_opt(secs, nsecs);
    let chrono::offset::LocalResult::Single(utc) = dt else {
        return Err(Error::InvalidData(format!(
            "Invalid timestamp: {}",
            name_hint
        )));
    };
    Ok(utc)
}

#[test]
fn test_timestamp_to_utc() {
    let result = timestamp_to_utc(1690985963000, "DTSTART").unwrap();
    assert_eq!(
        result,
        Utc.with_ymd_and_hms(2023, 8, 2, 14, 19, 23).unwrap()
    );
}

pub(crate) fn format_utc_msec(
    msec: Timestamp,
    tz: &chrono_tz::Tz,
    all_day: bool,
    name_hint: &str,
) -> Result<String, Error> {
    let utc = timestamp_to_utc(msec, name_hint)?;
    if all_day {
        return Ok(utc.format("%Y%m%d").to_string());
    }
    let tz_aware = utc.with_timezone(tz);
    Ok(tz_aware
        .format(ical::generator::ICAL_DATE_FORMAT)
        .to_string())
}

#[test]
fn test_format_utc_msec() {
    let tz = chrono_tz::Tz::from_str("Europe/Berlin").unwrap();
    let result = format_utc_msec(1693276200000, &tz, false, "DTSTART").unwrap();
    assert_eq!(result, "20230829T043000");

    let result = format_utc_msec(1693276200000, &tz, true, "DTSTART").unwrap();
    assert_eq!(result, "20230829");
}

fn local_time_to_utc_ms(time_str: &str, tz: &chrono_tz::Tz, name_hint: &str) -> Result<i64, Error> {
    let local = chrono::NaiveDateTime::parse_from_str(time_str, "%Y%m%dT%H%M%S");
    if let Err(err) = local {
        return Err(Error::InvalidData(format!(
            "Invalid timestamp: {}: {}",
            name_hint, err
        )));
    }
    let local = local.unwrap();
    let Some(tz_aware) = tz.from_local_datetime(&local).earliest() else {
        return Err(Error::InvalidData(format!(
            "Invalid timestamp: {}",
            name_hint
        )));
    };
    Ok(tz_aware.timestamp_millis())
}

#[test]
fn test_local_time_to_utc_ms() {
    let tz = chrono_tz::Tz::from_str("Europe/Berlin").unwrap();
    let result = local_time_to_utc_ms("20230829T043000", &tz, "DTSTART").unwrap();
    assert_eq!(result, 1693276200000);

    let result = local_time_to_utc_ms("20230829T043000Z", &tz, "DTSTART");
    assert!(result.is_err());

    let result = local_time_to_utc_ms("20230829", &tz, "DTSTART");
    assert!(result.is_err());
}

fn from_ical_get_timestamp(
    prop: &ical::property::Property,
    prop_hint: &str,
) -> Result<(Timestamp, bool), Error> {
    let Some(value) = &prop.value else {
        return Err(Error::InvalidData(format!("Missing value: {}", prop_hint)));
    };
    let Some(params) = &prop.params else {
        let utc = NaiveDateTime::parse_from_str(value.as_str(), "%Y%m%dT%H%M%SZ");
        if let Err(err) = utc {
            return Err(Error::InvalidData(format!(
                "invalid utc date: {}: {}",
                prop_hint, err
            )));
        }
        let utc = utc.unwrap();
        let ms = utc.and_utc().timestamp_millis();
        return Ok((ms, false));
    };
    for (param_name, param_values) in params {
        match param_name.as_str() {
            // params=Some([("VALUE", ["DATE"])]),
            "VALUE" => {
                if param_values.contains(&"DATE".to_string()) {
                    let local = NaiveDate::parse_from_str(value.as_str(), "%Y%m%d");
                    if let Err(err) = local {
                        return Err(Error::InvalidData(format!(
                            "invalid date: {}: {}",
                            prop_hint, err
                        )));
                    }
                    let local = local.unwrap();
                    let Some(dt) = local.and_hms_opt(0, 0, 0) else {
                        return Err(Error::InvalidData(format!(
                            "invalid date-time: {}",
                            prop_hint
                        )));
                    };
                    let ms = dt.and_utc().timestamp_millis();
                    return Ok((ms, true));
                }
            }
            // params=Some([("TZID", ["Asia/Novosibirsk"])]),
            "TZID" => {
                if param_values.is_empty() {
                    return Err(Error::InvalidData(format!(
                        "timezone not set: {}",
                        prop_hint
                    )));
                }
                let tzid = param_values[0].as_str();
                let tz = chrono_tz::Tz::from_str(tzid);
                if let Err(err) = tz {
                    return Err(Error::InvalidData(format!(
                        "invalid timezone: {}: {}",
                        prop_hint, err
                    )));
                }
                let tz = tz.unwrap();
                let ms = local_time_to_utc_ms(value.as_str(), &tz, prop_hint)?;
                return Ok((ms, false));
            }
            _ => {}
        }
    }
    Err(Error::InvalidData(format!(
        "Unknown timestamp format value: {}",
        prop_hint
    )))
}

#[test]
fn test_from_ical_get_timestamp() {
    let prop = ical_property!("DTSTAMP", "20230802T141923Z");
    let result = from_ical_get_timestamp(&prop, "DTSTAMP").unwrap();
    assert_eq!(result.0, 1690985963000);
    assert!(!result.1); // Not all day

    let prop = ical_property!(
        "DTSTART",
        "20230829T043000",
        ical_param!("TZID", "Europe/Berlin")
    );
    let result = from_ical_get_timestamp(&prop, "DTSTART").unwrap();
    assert_eq!(result.0, 1693276200000);
    assert!(!result.1); // Not all day

    let prop = ical_property!("DTSTART", "20230829", ical_param!("VALUE", "DATE"));
    let result = from_ical_get_timestamp(&prop, "DTSTART").unwrap();
    assert_eq!(result.0, 1693267200000);
    assert!(result.1); // All day
}

pub(crate) fn from_ical_get_event_bounds(
    event: &IcalEvent,
) -> Result<(Timestamp, Timestamp, bool), Error> {
    let prop = event
        .get_property("DTSTART")
        .ok_or_else(|| Error::InvalidData("Missing property: DTSTART".into()))?;
    let (start_ts, start_all_day) = from_ical_get_timestamp(prop, "DTSTART")?;

    // Try DTEND first, fall back to DURATION if DTEND is not present
    let (end_ts, end_all_day) = if let Some(prop) = event.get_property("DTEND") {
        from_ical_get_timestamp(prop, "DTEND")?
    } else if let Some(prop) = event.get_property("DURATION") {
        let Some(duration_str) = &prop.value else {
            return Err(Error::InvalidData("Missing DURATION value".to_string()));
        };
        let duration = parse_duration(duration_str)?;
        let all_day = duration.num_seconds() % 86400 == 0;
        let ts = start_ts + duration.num_milliseconds();
        (ts, all_day)
    } else {
        return Err(Error::InvalidData(
            "Missing property: DTEND or DURATION".into(),
        ));
    };
    // RFC 5545 Section 3.6.1:
    // The "VEVENT" is also the calendar component used to specify an
    // anniversary or daily reminder within a calendar.  These events
    // have a DATE value type for the "DTSTART" property instead of the
    // default value type of DATE-TIME.  If such a "VEVENT" has a "DTEND"
    // property, it MUST be specified as a DATE value also.  The
    // anniversary type of "VEVENT" can span more than one date (i.e.,
    // "DTEND" property value is set to a calendar date after the
    // "DTSTART" property value).  If such a "VEVENT" has a "DURATION"
    // property, it MUST be specified as a "dur-day" or "dur-week" value.
    if start_all_day && !end_all_day {
        return Err(Error::InvalidData("Invalid ebent end timestamp".into()));
    }

    Ok((start_ts, end_ts, start_all_day))
}

#[test]
fn test_from_ical_get_event_bounds() {
    let mut event = IcalEvent::new();
    event
        .properties
        .push(ical_property!("DTSTART", "20230802T141923Z"));
    event.properties.push(ical_property!("DURATION", "PT2H"));
    let (start, end, all_day) = from_ical_get_event_bounds(&event).unwrap();
    assert_eq!(start, 1690985963000);
    assert_eq!(end, 1690993163000);
    assert!(!all_day);

    // Should use DTEND value
    let mut event = IcalEvent::new();
    event
        .properties
        .push(ical_property!("DTSTART", "20230802T141923Z"));
    event.properties.push(ical_property!("DURATION", "PT2H"));
    event
        .properties
        .push(ical_property!("DTEND", "20230802T171923Z"));
    let (start, end, all_day) = from_ical_get_event_bounds(&event).unwrap();
    assert_eq!(start, 1690985963000);
    assert_eq!(end, 1690996763000);
    assert!(!all_day);

    // DTEND only
    let mut event = IcalEvent::new();
    event
        .properties
        .push(ical_property!("DTSTART", "20230802T141923Z"));
    event
        .properties
        .push(ical_property!("DTEND", "20230802T181923Z"));
    let (start, end, all_day) = from_ical_get_event_bounds(&event).unwrap();
    assert_eq!(start, 1690985963000);
    assert_eq!(end, 1691000363000);
    assert!(!all_day);

    // All-day event with DTEND
    let mut event = IcalEvent::new();
    event.properties.push(ical_property!(
        "DTSTART",
        "20230802",
        ical_param!("VALUE", "DATE")
    ));
    event.properties.push(ical_property!(
        "DTEND",
        "20230803",
        ical_param!("VALUE", "DATE")
    ));
    let (_, _, all_day) = from_ical_get_event_bounds(&event).unwrap();
    assert!(all_day);

    // All-day event with DURATION
    let mut event = IcalEvent::new();
    event.properties.push(ical_property!(
        "DTSTART",
        "20230802",
        ical_param!("VALUE", "DATE")
    ));
    event.properties.push(ical_property!("DURATION", "P2D"));
    let (_, _, all_day) = from_ical_get_event_bounds(&event).unwrap();
    assert!(all_day);

    // All-day event with wrong DURATION (not a days number)
    let mut event = IcalEvent::new();
    event.properties.push(ical_property!(
        "DTSTART",
        "20230802",
        ical_param!("VALUE", "DATE")
    ));
    event.properties.push(ical_property!("DURATION", "P2H"));
    let res = from_ical_get_event_bounds(&event);
    assert!(res.is_err());

    // All-day event with wrong DTEND (not a date)
    let mut event = IcalEvent::new();
    event.properties.push(ical_property!(
        "DTSTART",
        "20230802",
        ical_param!("VALUE", "DATE")
    ));
    event
        .properties
        .push(ical_property!("DTEND", "20230802T181923Z"));
    let res = from_ical_get_event_bounds(&event);
    assert!(res.is_err());
}

pub(crate) fn from_ical_get_timestamp_required(
    event: &IcalEvent,
    prop_name: &str,
) -> Result<Timestamp, Error> {
    let prop = event
        .get_property(prop_name)
        .ok_or_else(|| Error::InvalidData(format!("Missing prop: {}", prop_name)))?;
    let (ts, _) = from_ical_get_timestamp(prop, prop_name)?;
    Ok(ts)
}

#[test]
fn test_from_ical_get_timestamp_required() {
    let mut event = IcalEvent::new();
    event
        .properties
        .push(ical_property!("DTSTART", "20230802T141923Z"));
    let result = from_ical_get_timestamp_required(&event, "DTSTART").unwrap();
    assert_eq!(result, 1690985963000);

    let result = from_ical_get_timestamp_required(&event, "DTEND");
    assert!(result.is_err());
}

pub(crate) fn from_ical_get_exdate(
    ical_event: &IcalEvent,
) -> Result<Option<Vec<Timestamp>>, Error> {
    let mut exdate = Vec::new();
    for prop in &ical_event.properties {
        if prop.name == "EXDATE" {
            let (dt, _) = from_ical_get_timestamp(prop, "EXDATE")?;
            exdate.push(dt);
        }
    }
    Ok(if exdate.is_empty() {
        None
    } else {
        Some(exdate)
    })
}

#[test]
fn test_from_ical_get_exdate() {
    let mut event = IcalEvent::new();
    event.properties.push(ical_property!(
        "EXDATE",
        "20230829",
        ical_param!("VALUE", "DATE")
    ));
    event.properties.push(ical_property!(
        "EXDATE",
        "20230830",
        ical_param!("VALUE", "DATE")
    ));
    let result = from_ical_get_exdate(&event).unwrap();
    assert_eq!(result, Some(vec![1693267200000, 1693353600000]));

    let mut event = IcalEvent::new();
    event.properties.push(ical_property!("EXDATE", "20230829"));
    let result = from_ical_get_exdate(&event);
    assert!(result.is_err());

    let event = IcalEvent::new();
    let result = from_ical_get_exdate(&event).unwrap();
    assert_eq!(result, None);
}

pub(crate) fn from_ical_get_timezone(event_obj: &EventObject) -> Result<Option<String>, Error> {
    if event_obj.timezones.len() > 1 {
        return Err(Error::InvalidData(
            "multiple timezones not supported".into(),
        ));
    }
    let tzids: Vec<String> = event_obj.timezones.keys().cloned().collect();
    Ok(tzids.first().cloned())
}

#[test]
fn test_from_ical_get_timezone() {
    let event = EventObject {
        event: IcalEvent::new(),
        timezones: HashMap::from([("Europe/Berlin".to_string(), IcalTimeZone::new())]),
    };
    let result = from_ical_get_timezone(&event).unwrap();
    assert_eq!(result, Some("Europe/Berlin".to_string()));

    let event = EventObject {
        event: IcalEvent::new(),
        timezones: HashMap::from([
            ("Europe/Berlin".to_string(), IcalTimeZone::new()),
            ("Asia/Novosibirsk".to_string(), IcalTimeZone::new()),
        ]),
    };
    let result = from_ical_get_timezone(&event);
    assert!(result.is_err());
}

/*
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
*/

pub(crate) fn format_duration_rfc5545(milliseconds: i64) -> String {
    let mut result = String::from("P");

    if milliseconds < 0 {
        result.insert(0, '-');
    }

    let milliseconds = milliseconds.abs();
    let seconds = milliseconds / 1000;
    let minutes = seconds / 60;
    let hours = minutes / 60;
    let days = hours / 24;
    let mut has_t = false;

    let hours_remainder = hours % 24;
    let minutes_remainder = minutes % 60;
    let seconds_remainder = seconds % 60;

    if days > 0 {
        result.push_str(&format!("{}D", days));
    }
    if hours_remainder > 0 {
        if !has_t {
            result.push_str("T");
            has_t = true;
        }
        result.push_str(&format!("{}H", hours_remainder));
    }
    if minutes_remainder > 0 {
        if !has_t {
            result.push_str("T");
            has_t = true;
        }
        result.push_str(&format!("{}M", minutes_remainder));
    }
    if seconds_remainder > 0 || (days == 0 && hours_remainder == 0 && minutes_remainder == 0) {
        if !has_t {
            result.push_str("T");
        }
        result.push_str(&format!("{}S", seconds_remainder));
    }
    result
}

#[test]
fn test_format_duration_rfc5545() {
    assert_eq!(format_duration_rfc5545(3661000), "PT1H1M1S");
    assert_eq!(format_duration_rfc5545(86400000), "P1D");
    assert_eq!(format_duration_rfc5545(86401000), "P1DT1S");
    assert_eq!(format_duration_rfc5545(-3600000), "-PT1H");
    assert_eq!(format_duration_rfc5545(0), "PT0S");
}
