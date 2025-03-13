use crate::api::RecurringRule;
use crate::convert_time::timestamp_to_utc;
use rustical_store::Error;

pub(crate) fn parse_rrule_string(rrules: &str) -> Result<Option<Vec<RecurringRule>>, Error> {
    let rules = rrules
        .split('\n')
        .filter(|s| !s.is_empty())
        .map(RecurringRule::from_rrule_string)
        .collect::<Result<Vec<_>, _>>();
    if let Err(err) = rules {
        return Err(Error::InvalidData(format!("Invalid RRULE: {}", err)));
    }
    let rules = rules.unwrap();
    Ok(if rules.is_empty() { None } else { Some(rules) })
}

impl RecurringRule {
    pub(crate) fn to_rrule_string(&self) -> Result<String, Error> {
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
        if let Some(by_month) = &self.by_month {
            parts.push(format!(
                "BYMONTH={}",
                by_month
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        if let Some(by_day) = &self.by_day {
            parts.push(format!("BYDAY={}", by_day.join(",")));
        }
        if let Some(by_month_day) = &self.by_month_day {
            parts.push(format!(
                "BYMONTHDAY={}",
                by_month_day
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        if let Some(by_year_day) = &self.by_year_day {
            parts.push(format!(
                "BYYEARDAY={}",
                by_year_day
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        if let Some(by_week_no) = &self.by_week_no {
            parts.push(format!(
                "BYWEEKNO={}",
                by_week_no
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        if let Some(by_hour) = &self.by_hour {
            parts.push(format!(
                "BYHOUR={}",
                by_hour
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        if let Some(by_minute) = &self.by_minute {
            parts.push(format!(
                "BYMINUTE={}",
                by_minute
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        if let Some(by_second) = &self.by_second {
            parts.push(format!(
                "BYSECOND={}",
                by_second
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        if let Some(by_set_pos) = &self.by_set_pos {
            parts.push(format!(
                "BYSETPOS={}",
                by_set_pos
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
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
                        return Err(format!("Invalid date UNTIL: {}", err));
                    }
                    rule.end_date = Some(dt.unwrap().and_utc().timestamp_millis())
                }
                "COUNT" => {
                    rule.count = Some(value.parse().map_err(|e| format!("Invalid COUNT: {}", e))?);
                }
                "INTERVAL" => {
                    rule.interval = Some(
                        value
                            .parse()
                            .map_err(|e| format!("Invalid INTERVAL: {}", e))?,
                    );
                }
                "BYSECOND" => {
                    rule.by_second = Some(
                        value
                            .split(',')
                            .map(|s| s.parse::<u8>())
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(|e| format!("Invalid BYSECOND: {}", e))?,
                    );
                }
                "BYMINUTE" => {
                    rule.by_minute = Some(
                        value
                            .split(',')
                            .map(|s| s.parse::<u8>())
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(|e| format!("Invalid BYMINUTE: {}", e))?,
                    );
                }
                "BYHOUR" => {
                    rule.by_hour = Some(
                        value
                            .split(',')
                            .map(|s| s.parse::<u8>())
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(|e| format!("Invalid BYHOUR: {}", e))?,
                    );
                }
                "BYDAY" => {
                    rule.by_day = Some(value.split(',').map(|s| s.to_string()).collect());
                }
                "BYMONTHDAY" => {
                    rule.by_month_day = Some(
                        value
                            .split(',')
                            .map(|s| s.parse::<u8>())
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(|e| format!("Invalid BYMONTHDAY: {}", e))?,
                    );
                }
                "BYYEARDAY" => {
                    rule.by_year_day = Some(
                        value
                            .split(',')
                            .map(|s| s.parse::<u16>())
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(|e| format!("Invalid BYYEARDAY: {}", e))?,
                    );
                }
                "BYWEEKNO" => {
                    rule.by_week_no = Some(
                        value
                            .split(',')
                            .map(|s| s.parse::<i8>())
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(|e| format!("Invalid BYWEEKNO: {}", e))?,
                    );
                }
                "BYMONTH" => {
                    rule.by_month = Some(
                        value
                            .split(',')
                            .map(|s| s.parse::<u8>())
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(|e| format!("Invalid BYMONTH: {}", e))?,
                    );
                }
                "BYSETPOS" => {
                    rule.by_set_pos = Some(
                        value
                            .split(',')
                            .map(|s| s.parse::<i16>())
                            .collect::<Result<Vec<_>, _>>()
                            .map_err(|e| format!("Invalid BYSETPOS: {}", e))?,
                    );
                }
                "WKST" => {
                    rule.wkst = Some(value.split(',').map(|s| s.to_string()).collect());
                }
                _ => return Err(format!("Unknown part: {}", key)),
            }
        }
        if rule.freq.is_empty() {
            return Err("FREQ is required".to_string());
        }
        Ok(rule)
    }
}

#[test]
fn test_parse_rrule_string() {
    let rrule = "FREQ=DAILY;COUNT=10";
    let result = parse_rrule_string(rrule).unwrap().unwrap();
    assert_eq!(result.len(), 1);
    let rule = &result[0];
    assert_eq!(rule.freq, "DAILY");
    assert_eq!(rule.count, Some(10));
}

#[test]
fn test_parse_rrule_string_all_frequencies() {
    let frequencies = [
        "SECONDLY", "MINUTELY", "HOURLY", "DAILY", "WEEKLY", "MONTHLY", "YEARLY",
    ];
    for freq in frequencies {
        let rrule = format!("FREQ={}", freq);
        let result = parse_rrule_string(&rrule).unwrap().unwrap();
        assert_eq!(result[0].freq, freq);
    }
}

#[test]
fn test_parse_rrule_string_complex_rrule() {
    let rrule = "FREQ=MONTHLY;BYDAY=MO,TU,WE,TH,FR;BYSETPOS=-1";
    let result = parse_rrule_string(rrule).unwrap().unwrap();
    let rule = &result[0];
    assert_eq!(rule.freq, "MONTHLY");
    assert_eq!(
        rule.by_day,
        Some(vec![
            "MO".into(),
            "TU".into(),
            "WE".into(),
            "TH".into(),
            "FR".into()
        ])
    );
    assert_eq!(rule.by_set_pos, Some(vec![-1]));
}

#[test]
fn test_to_rrule_string() {
    let rule = RecurringRule {
        freq: "DAILY".into(),
        count: Some(10),
        ..Default::default()
    };
    let result = rule.to_rrule_string().unwrap();
    assert_eq!(result, "FREQ=DAILY;COUNT=10");
}

#[test]
fn test_to_rrule_string_complex_rrule() {
    let rule = RecurringRule {
        freq: "MONTHLY".into(),
        by_day: Some(vec![
            "MO".into(),
            "TU".into(),
            "WE".into(),
            "TH".into(),
            "FR".into(),
        ]),
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
