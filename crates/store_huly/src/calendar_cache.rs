use crate::api::{
    CLASS_EVENT, CLASS_RECURRING_EVENT, CLASS_RECURRING_INSTANCE, FindOptions, FindParams,
    HulyEvent, HulyEventData, HulyEventSlim, Timestamp, find_all,
};
use crate::auth::HulyUser;
use crate::convert::calc_etag;
use crate::sync_cache::SyncCache;
use rustical_store::calendar::CalendarObjectType;
use rustical_store::{Calendar, Error};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use std::vec;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    user: String,
    workspace: String,
}

impl CacheKey {
    fn new(user: &HulyUser) -> Result<Self, Error> {
        Ok(Self {
            user: user.id.clone(),
            workspace: user.workspace_url.clone(),
        })
    }
}

#[derive(Debug)]
pub struct HulyCalendarCache {
    calendars: HashMap<CacheKey, CachedCalendar>,
    users: HashMap<String, HulyUser>,
    invalidation_interval: Duration,
    sync_cache: Option<Arc<dyn SyncCache>>,
}

#[derive(Debug)]
struct CachedCalendar {
    hash: i64,
    fetched_at: SystemTime,
    calendar_id: String,
    events: Vec<(String, Timestamp)>,
}

impl HulyCalendarCache {
    pub fn new(invalidation_interval: Duration, sync_cache: Option<Arc<dyn SyncCache>>) -> Self {
        Self {
            calendars: HashMap::new(),
            users: HashMap::new(),
            invalidation_interval,
            sync_cache,
        }
    }

    /// Get the sync state for a given synctoken
    pub async fn get_sync_state(&self, synctoken: u64) -> Result<Vec<String>, Error> {
        if let Some(sync_cache) = &self.sync_cache {
            sync_cache.get_sync_state(synctoken).await
        } else {
            Ok(vec![])
        }
    }

    /// Set the sync state for a given synctoken
    pub async fn set_sync_state(
        &self,
        synctoken: u64,
        event_ids: Vec<String>,
    ) -> Result<(), Error> {
        if let Some(sync_cache) = &self.sync_cache {
            sync_cache.set_sync_state(synctoken, event_ids).await
        } else {
            Ok(())
        }
    }

    pub(crate) fn get_user(&self, user_id: &str, ws_url: Option<&str>) -> Result<HulyUser, Error> {
        if let Some(ws_url) = ws_url {
            self.users.get(format!("{}|{}", user_id, ws_url).as_str())
        } else {
            self.users.get(user_id)
        }
        .cloned()
        .ok_or(Error::UserNotFound)
    }

    pub(crate) fn try_get_user(&self, user_id: &str, ws_url: Option<&str>) -> Option<HulyUser> {
        if let Some(ws_url) = ws_url {
            self.users.get(format!("{}|{}", user_id, ws_url).as_str())
        } else {
            self.users.get(user_id)
        }
        .cloned()
    }

    pub(crate) fn set_user(&mut self, user_id: &str, ws_url: Option<&str>, user: HulyUser) {
        self.users.insert(
            if let Some(ws_url) = ws_url {
                format!("{}|{}", user_id, ws_url)
            } else {
                user_id.to_string()
            },
            user,
        );
    }

    pub(crate) fn invalidate(&mut self, user: &HulyUser) {
        let key = CacheKey::new(user).unwrap();
        self.calendars.remove(&key);
    }

    async fn add_entry(&mut self, user: &HulyUser) -> Result<&CachedCalendar, Error> {
        let key = CacheKey::new(user)?;
        self.calendars
            .insert(key.clone(), CachedCalendar::new(user).await?);
        Ok(self.calendars.get(&key).unwrap())
    }

    fn get_entry(&self, user: &HulyUser) -> Result<Option<&CachedCalendar>, Error> {
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
                    return Ok(Some(cal));
                }
            }
        }
        Ok(None)
    }

    pub(crate) async fn get_calendar_id(&mut self, user: &HulyUser) -> Result<String, Error> {
        let entry = if let Some(entry) = self.get_entry(user)? {
            entry
        } else {
            self.add_entry(user).await?
        };
        Ok(entry.calendar_id.clone())
    }

    pub(crate) async fn get_calendar(&mut self, user: &HulyUser) -> Result<Calendar, Error> {
        let entry = if let Some(entry) = self.get_entry(user)? {
            entry
        } else {
            self.add_entry(user).await?
        };
        let cal = Calendar {
            principal: user.id.to_string(),
            id: user.workspace_url.clone(),
            displayname: Some(user.workspace_url.clone()),
            order: 1,
            synctoken: entry.hash,
            description: None,
            color: None,
            timezone: None,
            timezone_id: None,
            deleted_at: None,
            subscription_url: None,
            push_topic: "".to_string(),
            components: vec![CalendarObjectType::Event],
        };
        Ok(cal)
    }

    pub(crate) async fn get_calendars(&mut self, user: &HulyUser) -> Result<Vec<Calendar>, Error> {
        let cals = user
            .workspaces
            .iter()
            .map(|ws| Calendar {
                principal: user.id.to_string(),
                id: ws.url.clone(),
                displayname: Some(ws.url.clone()),
                order: 1,
                synctoken: 0,
                description: None,
                color: None,
                timezone: None,
                timezone_id: None,
                deleted_at: None,
                subscription_url: None,
                push_topic: "".to_string(),
                components: vec![CalendarObjectType::Event],
            })
            .collect();
        Ok(cals)
    }

    pub(crate) async fn get_events(
        &mut self,
        user: &HulyUser,
    ) -> Result<Vec<(String, String)>, Error> {
        let entry = if let Some(entry) = self.get_entry(user)? {
            entry
        } else {
            self.add_entry(user).await?
        };
        Ok(entry
            .events
            .iter()
            .map(|(event_id, modified_on)| (event_id.clone(), calc_etag(event_id, *modified_on)))
            .collect())
    }

    pub(crate) async fn get_event(
        &mut self,
        user: &HulyUser,
        event_id: &str,
        include_recurrence: bool,
    ) -> Result<HulyEvent, Error> {
        //println!("\n### GET_EVENT {}\n", event_id);
        let entry = if let Some(entry) = self.get_entry(user)? {
            entry
        } else {
            self.add_entry(user).await?
        };
        let calendar_id = entry.calendar_id.clone();
        let params = FindParams {
            class: CLASS_EVENT,
            query: HashMap::from([("calendar", calendar_id.as_str()), ("eventId", event_id)]),
            options: None,
        };
        let mut events = find_all::<HulyEventData>(user, &params).await?;
        if events.is_empty() {
            return Err(Error::NotFound);
        }
        if events.len() > 1 {
            return Err(Error::InvalidData("multiple events found".into()));
        }
        let mut event = events.remove(0);
        event.event_id = Some(event_id.to_string());

        let mut instances = None;
        if include_recurrence && event.class == CLASS_RECURRING_EVENT {
            let params = FindParams {
                class: CLASS_RECURRING_INSTANCE,
                query: HashMap::from([
                    ("calendar", calendar_id.as_str()),
                    ("recurringEventId", event_id),
                ]),
                options: None,
            };
            let insts = find_all::<HulyEventData>(user, &params).await?;
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

    pub(crate) async fn get_synctoken(&mut self, user: &HulyUser) -> Result<i64, Error> {
        let entry = if let Some(entry) = self.get_entry(user)? {
            entry
        } else {
            self.add_entry(user).await?
        };
        Ok(entry.hash)
    }
}

impl CachedCalendar {
    async fn new(user: &HulyUser) -> Result<Self, Error> {
        let calendar_id = format!("{}_calendar", user.account_uuid);

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
        // let huly_calendars = find_all::<HulyCalendar>(&auth, params).await?;
        // if huly_calendars.is_empty() {
        //     println!("no huly calendars");
        //     return Err(Error::NotFound)
        // }
        // let calendar = huly_calendars[0].clone();
        //println!("*** huly calendar {}\n", serde_json::to_string_pretty(&calendar).unwrap());

        let params = FindParams {
            class: CLASS_EVENT,
            query: HashMap::from([("calendar", calendar_id.as_str())]),
            options: Some(FindOptions {
                projection: Some(HashMap::from([
                    ("eventId", 1),
                    ("modifiedOn", 1),
                    ("recurringEventId", 1),
                ])),
                sort: Some(HashMap::from([("eventId", 1)])),
            }),
        };
        let events = find_all::<HulyEventSlim>(user, &params).await?;
        //println!("*** HULY_EVENTS: {}", serde_json::to_string_pretty(&events).unwrap());
        if events.is_empty() {
            tracing::warn!("No events found for calendar {}", calendar_id);
        }

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

        let events = event_dates
            .into_iter()
            .map(|(id, dt)| (id, dt))
            .collect::<Vec<_>>();
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
