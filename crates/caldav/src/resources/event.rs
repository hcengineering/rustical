use crate::proptypes::write_string_prop;
use actix_web::{web::Data, HttpRequest};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rustical_auth::AuthInfo;
use rustical_dav::resource::Resource;
use rustical_store::calendar::CalendarStore;
use rustical_store::event::Event;
use std::sync::Arc;
use strum::{EnumString, VariantNames};
use tokio::sync::RwLock;

pub struct EventResource<C: CalendarStore + ?Sized> {
    pub cal_store: Arc<RwLock<C>>,
    pub path: String,
    pub event: Event,
}

#[derive(EnumString, Debug, VariantNames)]
#[strum(serialize_all = "kebab-case")]
pub enum EventProp {
    Getetag,
    CalendarData,
    Getcontenttype,
}

#[async_trait(?Send)]
impl<C: CalendarStore + ?Sized> Resource for EventResource<C> {
    type UriComponents = (String, String, String); // principal, calendar, event
    type MemberType = Self;
    type PropType = EventProp;

    fn get_path(&self) -> &str {
        &self.path
    }

    async fn get_members(&self) -> Result<Vec<Self::MemberType>> {
        Ok(vec![])
    }

    async fn acquire_from_request(
        req: HttpRequest,
        _auth_info: AuthInfo,
        uri_components: Self::UriComponents,
        _prefix: String,
    ) -> Result<Self> {
        let (_principal, cid, uid) = uri_components;

        let cal_store = req
            .app_data::<Data<RwLock<C>>>()
            .ok_or(anyhow!("no calendar store in app_data!"))?
            .clone()
            .into_inner();

        let event = cal_store.read().await.get_event(&cid, &uid).await?;

        Ok(Self {
            cal_store,
            event,
            path: req.path().to_string(),
        })
    }

    fn write_prop<W: std::io::Write>(
        &self,
        writer: &mut quick_xml::Writer<W>,
        prop: Self::PropType,
    ) -> Result<()> {
        match prop {
            EventProp::Getetag => {
                write_string_prop(writer, "getetag", &self.event.get_etag())?;
            }
            EventProp::CalendarData => {
                write_string_prop(writer, "C:calendar-data", &self.event.get_ics())?;
            }
            EventProp::Getcontenttype => {
                write_string_prop(writer, "getcontenttype", "text/calendar;charset=utf-8")?;
            }
        };
        Ok(())
    }
}
