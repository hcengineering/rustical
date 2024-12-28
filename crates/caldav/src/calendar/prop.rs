use rustical_xml::XmlSerialize;

#[derive(Debug, Clone, XmlSerialize, PartialEq)]
pub struct SupportedCalendarComponent {
    #[xml(ty = "attr")]
    pub name: String,
}

#[derive(Debug, Clone, XmlSerialize, PartialEq)]
pub struct SupportedCalendarComponentSet {
    // #[serde(rename = "C:comp")]
    #[xml(flatten)]
    pub comp: Vec<SupportedCalendarComponent>,
}

#[derive(Debug, Clone, XmlSerialize, PartialEq)]
pub struct CalendarData {
    #[xml(ty = "attr")]
    content_type: String,
    #[xml(ty = "attr")]
    version: String,
}

impl Default for CalendarData {
    fn default() -> Self {
        Self {
            content_type: "text/calendar".to_owned(),
            version: "2.0".to_owned(),
        }
    }
}

#[derive(Debug, Clone, XmlSerialize, Default, PartialEq)]
pub struct SupportedCalendarData {
    // #[serde(rename = "C:calendar-data", alias = "calendar-data")]
    calendar_data: CalendarData,
}

#[derive(Debug, Clone, XmlSerialize, PartialEq)]
pub enum ReportMethod {
    CalendarQuery,
    CalendarMultiget,
    SyncCollection,
}

#[derive(Debug, Clone, XmlSerialize, PartialEq)]
pub struct ReportWrapper {
    report: ReportMethod,
}

// RFC 3253 section-3.1.5
#[derive(Debug, Clone, XmlSerialize, PartialEq)]
pub struct SupportedReportSet {
    #[xml(flatten)]
    supported_report: Vec<ReportWrapper>,
}

impl Default for SupportedReportSet {
    fn default() -> Self {
        Self {
            supported_report: vec![
                ReportWrapper {
                    report: ReportMethod::CalendarQuery,
                },
                ReportWrapper {
                    report: ReportMethod::CalendarMultiget,
                },
                ReportWrapper {
                    report: ReportMethod::SyncCollection,
                },
            ],
        }
    }
}

#[derive(Debug, Clone, XmlSerialize, PartialEq)]
pub enum Transport {
    // #[serde(rename = "P:web-push")]
    WebPush,
}

#[derive(Debug, Clone, XmlSerialize, PartialEq)]
pub struct TransportWrapper {
    #[xml(ty = "untagged")]
    transport: Transport,
}

#[derive(Debug, Clone, XmlSerialize, PartialEq)]
pub struct Transports {
    // NOTE: Here we implement an older version of the spec since the new property name is not reflected
    // in DAVx5 yet
    // https://github.com/bitfireAT/webdav-push/commit/461259a2f2174454b2b00033419b11fac52b79e3
    // #[serde(rename = "P:transport")]
    #[xml(flatten, rename = b"transport")]
    transports: Vec<TransportWrapper>,
}

impl Default for Transports {
    fn default() -> Self {
        Self {
            transports: vec![TransportWrapper {
                transport: Transport::WebPush,
            }],
        }
    }
}
