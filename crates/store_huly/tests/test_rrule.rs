use store_huly::convert::{parse_rrule_string, RecurringRule};
use rustical_store::Error;

#[test]
fn test_parse_basic_rrule() {
    let rrule = "FREQ=DAILY;COUNT=10";
    let result = parse_rrule_string(rrule).unwrap().unwrap();
    assert_eq!(result.len(), 1);
    let rule = &result[0];
    assert_eq!(rule.freq, "DAILY");
    assert_eq!(rule.count, Some(10));
}

#[test]
fn test_parse_all_frequencies() {
    let frequencies = ["SECONDLY", "MINUTELY", "HOURLY", "DAILY", "WEEKLY", "MONTHLY", "YEARLY"];
    for freq in frequencies {
        let rrule = format!("FREQ={}", freq);
        let result = parse_rrule_string(&rrule).unwrap().unwrap();
        assert_eq!(result[0].freq, freq);
    }
}

#[test]
fn test_parse_complex_rrule() {
    let rrule = "FREQ=MONTHLY;BYDAY=MO,TU,WE,TH,FR;BYSETPOS=-1";
    let result = parse_rrule_string(rrule).unwrap().unwrap();
    let rule = &result[0];
    assert_eq!(rule.freq, "MONTHLY");
    assert_eq!(rule.by_day, Some(vec!["MO".into(), "TU".into(), "WE".into(), "TH".into(), "FR".into()]));
    assert_eq!(rule.by_set_pos, Some(vec![-1]));
}

#[test]
fn test_parse_invalid_rrule() {
    let invalid_cases = [
        "FREQ=INVALID",
        "COUNT=-1",
        "FREQ=DAILY;INVALID=VALUE",
        "BYDAY=INVALID",
    ];
    for case in invalid_cases {
        assert!(parse_rrule_string(case).is_err());
    }
}

#[test]
fn test_format_basic_rrule() {
    let rule = RecurringRule {
        freq: "DAILY".into(),
        count: Some(10),
        ..Default::default()
    };
    let result = rule.to_rrule_string().unwrap();
    assert_eq!(result, "FREQ=DAILY;COUNT=10");
}

#[test]
fn test_format_complex_rrule() {
    let rule = RecurringRule {
        freq: "MONTHLY".into(),
        by_day: Some(vec!["MO".into(), "TU".into(), "WE".into(), "TH".into(), "FR".into()]),
        by_set_pos: Some(vec![-1]),
        ..Default::default()
    };
    let result = rule.to_rrule_string().unwrap();
    assert_eq!(result, "FREQ=MONTHLY;BYDAY=MO,TU,WE,TH,FR;BYSETPOS=-1");
}

#[test]
fn test_roundtrip() {
    let cases = [
        "FREQ=YEARLY;BYMONTH=3;BYDAY=-1SU",
        "FREQ=MONTHLY;BYDAY=MO,TU,WE,TH,FR;BYSETPOS=-1",
        "FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE,FR",
        "FREQ=DAILY;COUNT=10",
    ];
    for case in cases {
        let parsed = parse_rrule_string(case).unwrap().unwrap();
        let formatted = parsed[0].to_rrule_string().unwrap();
        assert_eq!(case, formatted);
    }
}
