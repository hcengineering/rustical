use rustical_store_huly::convert::{from_ical_get_timestamp, from_ical_get_timestamps, parse_duration};
use ical::property::Property;
use ical::Event as IcalEvent;

#[test]
fn test_from_ical_get_timestamp_basic() {
    // Real example: UTC timestamp from event.ics DTSTAMP field
    let mut prop = Property::new();
    prop.name = "DTSTAMP".to_string();
    prop.value = Some("20230831T102923Z".to_string());
    let result = from_ical_get_timestamp(&prop, "DTSTAMP").unwrap();
    assert!(!result.1); // Not a DATE
    assert!(result.0.is_some());

    // Real example: Local time with TZID from event.ics DTSTART field
    let mut prop = Property::new();
    prop.name = "DTSTART".to_string();
    prop.value = Some("20230829T043000".to_string());
    let mut params = vec![("TZID".to_string(), vec!["Europe/Berlin".to_string()])];
    prop.params = Some(params);
    let result = from_ical_get_timestamp(&prop, "DTSTART").unwrap();
    assert!(!result.1); // Not a DATE
    assert!(result.0.is_some());

    // Real example: DATE value from timezone.ics DTSTART field
    let mut prop = Property::new();
    prop.name = "DTSTART".to_string();
    prop.value = Some("19700101".to_string());
    let mut params = vec![("VALUE".to_string(), vec!["DATE".to_string()])];
    prop.params = Some(params);
    let result = from_ical_get_timestamp(&prop, "DTSTART").unwrap();
    assert!(result.1); // Is a DATE
    assert!(result.0.is_some());
}

#[test]
fn test_from_ical_get_timestamp_errors() {
    // Invalid UTC format
    let mut prop = Property::new();
    prop.name = "DTSTART".to_string();
    prop.value = Some("2023-08-31T10:29:23Z".to_string()); // Wrong format (has dashes and colons)
    assert!(from_ical_get_timestamp(&prop, "test").is_err());

    // Invalid timezone
    let mut prop = Property::new();
    prop.name = "DTSTART".to_string();
    prop.value = Some("20230829T043000".to_string());
    let mut params = vec![("TZID".to_string(), vec!["Invalid/Zone".to_string()])];
    prop.params = Some(params);
    assert!(from_ical_get_timestamp(&prop, "test").is_err());

    // Missing value
    let mut prop = Property::new();
    prop.name = "DTSTART".to_string();
    assert!(from_ical_get_timestamp(&prop, "test").is_err());

    // Invalid date format with VALUE=DATE
    let mut prop = Property::new();
    prop.name = "DTSTART".to_string();
    prop.value = Some("2023-08-29".to_string()); // Wrong format (has dashes)
    let mut params = vec![("VALUE".to_string(), vec!["DATE".to_string()])];
    prop.params = Some(params);
    assert!(from_ical_get_timestamp(&prop, "test").is_err());
}
#[test]
fn test_duration_parsing() {
    assert_eq!(parse_duration("P1W").unwrap(), chrono::Duration::weeks(1));
    assert_eq!(parse_duration("P1D").unwrap(), chrono::Duration::days(1));
    assert_eq!(parse_duration("PT1H").unwrap(), chrono::Duration::hours(1));
    assert_eq!(parse_duration("P1DT2H").unwrap(), chrono::Duration::days(1) + chrono::Duration::hours(2));
    assert_eq!(parse_duration("-P1D").unwrap(), -chrono::Duration::days(1));
    assert_eq!(parse_duration("PT1H30M").unwrap(), chrono::Duration::hours(1) + chrono::Duration::minutes(30));
    assert_eq!(parse_duration("P1DT1H1M1S").unwrap(),
        chrono::Duration::days(1) + chrono::Duration::hours(1) +
        chrono::Duration::minutes(1) + chrono::Duration::seconds(1));

    assert!(parse_duration("1D").is_err());
    assert!(parse_duration("PT").is_err());
    assert!(parse_duration("P1H").is_err());
    assert!(parse_duration("PTT1H").is_err());
    assert!(parse_duration("P-1D").is_err());
    assert!(parse_duration("P1.5D").is_err());
}

#[test]
fn test_from_ical_get_timestamps_with_duration() {
    let mut event = IcalEvent::new();

    let mut prop = Property::new();
    prop.name = "DTSTART".to_string();
    prop.value = Some("20240101T100000Z".to_string());
    event.properties.push(prop);

    let mut prop = Property::new();
    prop.name = "DURATION".to_string();
    prop.value = Some("PT2H".to_string());
    event.properties.push(prop);

    let (start, end, all_day) = from_ical_get_timestamps(&event).unwrap();
    assert_eq!(start.unwrap(), 1704106800000);
    assert_eq!(end.unwrap(), 1704114000000);
    assert!(!all_day);

    let mut event = IcalEvent::new();
    let mut prop = Property::new();
    prop.name = "DURATION".to_string();
    prop.value = Some("PT2H".to_string());
    event.properties.push(prop);
    assert!(from_ical_get_timestamps(&event).is_err());

    let mut event = IcalEvent::new();
    let mut prop = Property::new();
    prop.name = "DTSTART".to_string();
    prop.value = Some("20240101T100000Z".to_string());
    event.properties.push(prop);

    let mut prop = Property::new();
    prop.name = "DURATION".to_string();
    prop.value = Some("-PT1H".to_string());
    event.properties.push(prop);

    let (start, end, all_day) = from_ical_get_timestamps(&event).unwrap();
    assert_eq!(start.unwrap(), 1704106800000);
    assert_eq!(end.unwrap(), 1704103200000);
    assert!(!all_day);

    // Test DTEND basic functionality
    let mut event = IcalEvent::new();
    let mut prop = Property::new();
    prop.name = "DTSTART".to_string();
    prop.value = Some("20240101T100000Z".to_string());
    event.properties.push(prop);

    let mut prop = Property::new();
    prop.name = "DTEND".to_string();
    prop.value = Some("20240101T120000Z".to_string());
    event.properties.push(prop);

    let (start, end, all_day) = from_ical_get_timestamps(&event).unwrap();
    assert_eq!(start.unwrap(), 1704106800000);
    assert_eq!(end.unwrap(), 1704114000000);
    assert!(!all_day);

    // Test DTEND takes precedence over DURATION
    let mut event = IcalEvent::new();
    let mut prop = Property::new();
    prop.name = "DTSTART".to_string();
    prop.value = Some("20240101T100000Z".to_string());
    event.properties.push(prop);

    let mut prop = Property::new();
    prop.name = "DTEND".to_string();
    prop.value = Some("20240101T120000Z".to_string());
    event.properties.push(prop);

    let mut prop = Property::new();
    prop.name = "DURATION".to_string();
    prop.value = Some("PT3H".to_string()); // Different duration than DTEND
    event.properties.push(prop);

    let (start, end, all_day) = from_ical_get_timestamps(&event).unwrap();
    assert_eq!(start.unwrap(), 1704106800000);
    assert_eq!(end.unwrap(), 1704114000000); // Should use DTEND value
    assert!(!all_day);

    // Test all-day event with DTEND
    let mut event = IcalEvent::new();
    let mut prop = Property::new();
    prop.name = "DTSTART".to_string();
    prop.value = Some("20240101".to_string());
    prop.params = Some(vec![("VALUE".to_string(), vec!["DATE".to_string()])]);
    event.properties.push(prop);

    let mut prop = Property::new();
    prop.name = "DTEND".to_string();
    prop.value = Some("20240102".to_string());
    prop.params = Some(vec![("VALUE".to_string(), vec!["DATE".to_string()])]);
    event.properties.push(prop);

    let (start, end, all_day) = from_ical_get_timestamps(&event).unwrap();
    assert!(all_day);
    assert_eq!(end.unwrap(), start.unwrap() + 86400000 - 1); // Should be adjusted by -1ms
}
