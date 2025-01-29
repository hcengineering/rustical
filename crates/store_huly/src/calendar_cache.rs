use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::{Duration, SystemTime};
use std::vec;
use rustical_store::auth::User;
use rustical_store::calendar::CalendarObjectType;
use rustical_store::{Calendar, Error};
use crate::api::{find_all, FindOptions, FindParams, HulyEvent, HulyEventSlim, HulyEventData, Timestamp,
    CLASS_EVENT, CLASS_RECURRING_EVENT, CLASS_RECURRING_INSTANCE};
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

#[derive(Debug)]
struct CachedCalendar {
    hash: i64,
    fetched_at: SystemTime,
    calendar_id: String,
    events: Vec<(String, Timestamp)>,
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

    pub(crate) fn api_url(&self) -> &str {
        &self.api_url
    }

    pub(crate) fn invalidate(&mut self, user: &User) {
        let key = CacheKey::new(user).unwrap();
        self.calendars.remove(&key);
    }

    async fn add_entry(&mut self, user: &User) -> Result<&CachedCalendar, Error> {
        let key = CacheKey::new(user)?;
        self.calendars.insert(key.clone(), CachedCalendar::new(&self.api_url, user).await?);
        Ok(self.calendars.get(&key).unwrap())
    }

    fn get_entry(&self, user: &User) -> Result<Option<&CachedCalendar>, Error> {
        // When updating, a client makes several calls in sequence
        // And almost in every call it asks for get_calendar()
        // This is not practical to send requests to Huly API at each client's call,
        // because all of them address the same data. 
        // So after the first call we cache the data for a short period of time
        // to make subsequent calls faster
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
        Ok(entry.events.iter().map(|(event_id, modified_on)| {
            (event_id.clone(), calc_etag(event_id, *modified_on))
        }).collect())
    }

    pub(crate) async fn get_event(&mut self, user: &User, event_id: &str) -> Result<HulyEventData, Error> {
        let entry = if let Some(entry) = self.get_entry(user)? {
            entry
        } else {
            self.add_entry(user).await?
        };
        let auth = user.try_into()?;
        let calendar_id = entry.calendar_id.clone();
        let params = FindParams {
            class: CLASS_EVENT,
            query: HashMap::from([
                ("calendar", calendar_id.as_str()),
                ("eventId", event_id),
            ]),
            options: None,
        };
        let mut events = find_all::<Vec<HulyEventData>>(&self.api_url, &auth, &params).await?;
        //println!("*** HULY_EVENTS:\n{}", serde_json::to_string_pretty(&events).unwrap());
        if events.is_empty() {
            return Err(Error::NotFound)
        }
        let mut event = events.remove(0);
        event.event_id = Some(event_id.to_string());
        //println!("*** huly event: {}", serde_json::to_string_pretty(&event).unwrap());
        Ok(event)
    }

    pub(crate) async fn get_event_ex(&mut self, user: &User, event_id: &str) -> Result<HulyEvent, Error> {
        println!("\n### GET_EVENT {}\n", event_id);
        let entry = if let Some(entry) = self.get_entry(user)? {
            entry
        } else {
            self.add_entry(user).await?
        };
        let auth = user.try_into()?;
        let calendar_id = entry.calendar_id.clone();
        let params = FindParams {
            class: CLASS_EVENT,
            query: HashMap::from([
                ("calendar", calendar_id.as_str()),
                ("eventId", event_id),
            ]),
            options: None,
        };
        let mut events = find_all::<Vec<HulyEventData>>(&self.api_url, &auth, &params).await?;
        if events.is_empty() {
            return Err(Error::NotFound)
        }
        if events.len() > 1 {
            return Err(Error::InvalidData("multiple events found".into()))
        }
        let mut event = events.remove(0);
        event.event_id = Some(event_id.to_string());

        let mut instances = None;
        if event.class == CLASS_RECURRING_EVENT {
            let params = FindParams {
                class: CLASS_RECURRING_INSTANCE,
                query: HashMap::from([
                    ("calendar", calendar_id.as_str()),
                    ("recurringEventId", event_id),
                ]),
                options: None,
            };
            let insts = find_all::<Vec<HulyEventData>>(&self.api_url, &auth, &params).await?;
            if !insts.is_empty() {
                instances = Some(insts);
            }
        }

        let event = HulyEvent {
            data: event,
            instances,
        };

        Ok(event)
    }

}

impl CachedCalendar {
    async fn new(api_url: &str, user: &User) -> Result<Self, Error> {
        let Some(account) = &user.account else {
            return Err(Error::ApiError("No account".into()))
        };

        let auth = user.try_into()?;
        let calendar_id = format!("{}_calendar", account);

        // let params = FindParams {
        //     class: "calendar:class:Calendar".to_string(),
        //     query: HashMap::from([("_id".to_string(), calendar_id)]),
        //     options: Some(FindOptions{
        //         projection: Some(HashMap::from([
        //             ("_id".to_string(), 1), 
        //             ("modifiedOn".to_string(), 1)
        //         ])),
        //     }),
        // };
        // let huly_calendars = find_all::<Vec<HulyCalendar>>(api_url, &auth, params).await?;
        // if huly_calendars.is_empty() {
        //     println!("no huly calendars");
        //     return Err(Error::NotFound)
        // }
        // let calendar = huly_calendars[0].clone();
        //println!("*** huly calendar {}\n", serde_json::to_string_pretty(&calendar).unwrap());

        let params = FindParams {
            class: CLASS_EVENT,
            query: HashMap::from([("calendar", calendar_id.as_str())]),
            options: Some(FindOptions{
                projection: Some(HashMap::from([
                    ("eventId", 1),
                    ("modifiedOn", 1),
                    ("recurringEventId", 1),
                ])),
            }),
        };
        let events = find_all::<Vec<HulyEventSlim>>(api_url, &auth, &params).await?;
        // println!("*** huly events: {}", serde_json::to_string_pretty(&events).unwrap());

        let mut event_dates = HashMap::new();
        for event in &events {
            if let Some(recurring_event_id) = &event.recurring_event_id {
                match event_dates.entry(recurring_event_id.clone()) {
                    Entry::Occupied(mut entry) => {
                        if event.modified_on > *entry.get() {
                            *entry.get_mut() = event.modified_on;
                        }
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(event.modified_on);
                    }
                }
            } else {
                event_dates.insert(event.event_id.clone(), event.modified_on);
            }
        }

        let events = event_dates.into_iter().map(|(id, dt)| (id, dt)).collect::<Vec<_>>();
        let mut s = DefaultHasher::new();
        events.hash(&mut s);
        let hash = s.finish();

        Ok(Self {
            hash: hash as i64,
            fetched_at: SystemTime::now(),
            calendar_id,
            events,
        })
    }
}
