use rustical_store_huly::convert::from_ical_get_timestamp;
use ical::property::Property;

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
