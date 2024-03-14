use crate::tagname::TagName;
use actix_web::{web::Data, HttpRequest};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rustical_auth::AuthInfo;
use rustical_dav::{resource::Resource, xml_snippets::TextElement};
use rustical_store::calendar::CalendarStore;
use rustical_store::event::Event;
use std::sync::Arc;
use strum::{EnumProperty, EnumString, IntoStaticStr, VariantNames};
use tokio::sync::RwLock;

pub struct EventResource<C: CalendarStore + ?Sized> {
    pub cal_store: Arc<RwLock<C>>,
    pub path: String,
    pub event: Event,
}

#[derive(EnumString, Debug, VariantNames, IntoStaticStr, EnumProperty)]
#[strum(serialize_all = "kebab-case")]
pub enum EventProp {
    Getetag,
    #[strum(props(tagname = "C:calendar-data"))]
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
                writer.write_serializable(
                    prop.tagname(),
                    &TextElement(Some(self.event.get_etag())),
                )?;
            }
            EventProp::CalendarData => {
                writer
                    .write_serializable(prop.tagname(), &TextElement(Some(self.event.get_ics())))?;
            }
            EventProp::Getcontenttype => {
                writer.write_serializable(
                    prop.tagname(),
                    &TextElement(Some("text/calendar;charset=utf-8".to_owned())),
                )?;
            }
        };
        Ok(())
    }
}
