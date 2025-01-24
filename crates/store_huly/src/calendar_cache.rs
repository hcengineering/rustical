use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::{Duration, SystemTime};
use rustical_store::auth::User;
use rustical_store::calendar::CalendarObjectType;
use rustical_store::{Calendar, Error};
use crate::api::{find_all, FindOptions, FindParams, HulyCalendar, HulyEvent, HulyEventData};
use crate::auth::get_workspaces;
use crate::convert::calc_etag;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    user: String,
    workspace: String,
}

impl CacheKey {
    fn new(user: &User) -> Result<Self, Error> {
        if user.workspace.is_none() {
            return Err(Error::ApiError("no workspace".into()))
        }
        Ok(Self {
            user: user.id.clone(),
            workspace: user.workspace.clone().unwrap_or_default(),
        })
    }
}

#[derive(Debug)]
pub struct HulyCalendarCache {
    api_url: String,
    accounts_url: String,
    calendars: HashMap<CacheKey, CachedCalendar>,
    invalidation_interval: Duration,
}

impl HulyCalendarCache {
    pub fn new(api_url: String, accounts_url: String, invalidation_interval: Duration) -> Self {
        Self {
            api_url,
            accounts_url,
            calendars: HashMap::new(),
            invalidation_interval,
        }
    }
}

#[derive(Debug)]
struct CachedCalendar {
    hash: i64,
    fetched_at: SystemTime,
    calendar_id: String,
    events: Vec<HulyEvent>,
}

impl HulyCalendarCache {
    async fn add_entry(&mut self, user: &User) -> Result<&CachedCalendar, Error> {
        let key = CacheKey::new(user)?;
        self.calendars.insert(key.clone(), CachedCalendar::new(&self.api_url, user).await?);
        Ok(self.calendars.get(&key).unwrap())
    }

    fn get_entry(&self, user: &User) -> Result<Option<&CachedCalendar>, Error> {
        let key = CacheKey::new(user)?;
        if let Some(cal) = self.calendars.get(&key) {
            if let Ok(elapsed) = cal.fetched_at.elapsed() {
                if elapsed < self.invalidation_interval {
                    return Ok(Some(cal))
                }
            }
        }
        Ok(None)
    }

    pub(crate) async fn get_calendar_id(&mut self, user: &User) -> Result<String, Error> {
        let entry = if let Some(entry) = self.get_entry(user)? {
            entry
        } else {
            self.add_entry(user).await?
        };
        Ok(entry.calendar_id.clone())
    }

    pub(crate) async fn get_calendar(&mut self, user: &User) -> Result<Calendar, Error> {
        let entry = if let Some(entry) = self.get_entry(user)? {
            entry
        } else {
            self.add_entry(user).await?
        };
        let cal = Calendar {
            principal: user.id.to_string(),
            id: user.workspace.clone().unwrap_or_default(),
            displayname: Some(user.workspace.clone().unwrap_or_default()),
            order: 1,
            synctoken: entry.hash,
            description: None,
            color: None,
            timezone: None,
            timezone_id: None,
            deleted_at: None,
            subscription_url: None,
            push_topic: "".to_string(),
            components: vec![CalendarObjectType::Event]
        };
        Ok(cal)
    }

    pub(crate) async fn get_calendars(&mut self, user: &User) -> Result<Vec<Calendar>, Error> {
        if user.workspace.is_some() {
            return Err(Error::ApiError("Workspace already selected".into()));
        }
        let Some(token) = &user.password else {
            return Err(Error::ApiError("Unauthorized".into()));
        };
        let workspaces = get_workspaces(&self.accounts_url, token).await?;
        let cals = workspaces.into_iter().map(|ws| Calendar {
            principal: user.id.to_string(),
            id: ws.clone(),
            displayname: Some(ws),
            order: 1,
            synctoken: 0,
            description: None,
            color: None,
            timezone: None,
            timezone_id: None,
            deleted_at: None,
            subscription_url: None,
            push_topic: "".to_string(),
            components: vec![CalendarObjectType::Event]
        }).collect();
        Ok(cals)
    }

    pub(crate) async fn get_events(&mut self, user: &User) -> Result<Vec<(String, String)>, Error> {
        let entry = if let Some(entry) = self.get_entry(user)? {
            entry
        } else {
            self.add_entry(user).await?
        };
        Ok(entry.events.iter().map(|e| {
            (e.event_id.clone(), calc_etag(&e.event_id, e.modified_on))
        }).collect())
    }

    pub(crate) async fn get_event(&mut self, user: &User, event_id: &str) -> Result<HulyEventData, Error> {
        let entry = if let Some(entry) = self.get_entry(user)? {
            entry
        } else {
            self.add_entry(user).await?
        };
        let query = FindParams {
            class: "calendar:class:Event".to_string(),
            query: HashMap::from([
                ("calendar".to_string(), entry.calendar_id.clone()),
                ("eventId".to_string(), event_id.to_string()),
            ]),
            options: None,
        };
        let mut events = find_all::<Vec<HulyEventData>>(&self.api_url, user.try_into()?, query).await?;
        if events.is_empty() {
            return Err(Error::NotFound)
        }
        if events.len() > 1 {
            return Err(Error::InvalidData("multiple events found".into()))
        }
        let mut event = events.remove(0);
        event.event_id = Some(event_id.to_string());
        println!("*** huly event: {}", serde_json::to_string_pretty(&event).unwrap());
        Ok(event)
    }
}

impl CachedCalendar {
    async fn new(api_url: &str, user: &User) -> Result<Self, Error> {
        let query = FindParams {
            class: "calendar:class:Calendar".to_string(),
            query: HashMap::from([("name".to_string(), user.id.to_string())]),
            options: Some(FindOptions{
                projection: Some(HashMap::from([
                    ("_id".to_string(), 1), 
                    ("modifiedOn".to_string(), 1)
                ])),
            }),
        };
        let huly_calendars = find_all::<Vec<HulyCalendar>>(api_url, user.try_into()?, query).await?;
        if huly_calendars.is_empty() {
            println!("no huly calendars");
            return Err(Error::NotFound)
        }
        let calendar = huly_calendars[0].clone();
        //println!("*** huly calendar {}\n", serde_json::to_string_pretty(&calendar).unwrap());

        let query = FindParams {
            class: "calendar:class:Event".to_string(),
            query: HashMap::from([("calendar".to_string(), calendar.id.clone())]),
            options: Some(FindOptions{
                projection: Some(HashMap::from([
                    ("eventId".to_string(), 1),
                    ("modifiedOn".to_string(), 1),
                ])),
            }),
        };
        let events = find_all::<Vec<HulyEvent>>(api_url, user.try_into()?, query).await?;
        // println!("*** huly events: {}", serde_json::to_string_pretty(&events).unwrap());

        let hashed = events.iter().map(|e| (&e.event_id, e.modified_on)).collect::<Vec<_>>();
        let mut s = DefaultHasher::new();
        hashed.hash(&mut s);
        let hash = s.finish();

        Ok(Self {
            hash: hash as i64,
            fetched_at: SystemTime::now(),
            calendar_id: calendar.id,
            events,
        })
    }
}
