use std::sync::Arc;

use crate::proptypes::write_href_prop;
use actix_web::{web::Data, HttpRequest};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use quick_xml::events::BytesText;
use rustical_auth::AuthInfo;
use rustical_dav::{resource::Resource, xml_snippets::write_resourcetype};
use rustical_store::calendar::CalendarStore;
use strum::{EnumString, IntoStaticStr, VariantNames};
use tokio::sync::RwLock;

use super::calendar::CalendarResource;

pub struct PrincipalCalendarsResource<C: CalendarStore + ?Sized> {
    prefix: String,
    principal: String,
    path: String,
    cal_store: Arc<RwLock<C>>,
}

#[derive(EnumString, Debug, VariantNames, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
pub enum PrincipalProp {
    Resourcetype,
    CurrentUserPrincipal,
    #[strum(serialize = "principal-URL")]
    PrincipalUrl,
    CalendarHomeSet,
    CalendarUserAddressSet,
}

#[async_trait(?Send)]
impl<C: CalendarStore + ?Sized> Resource for PrincipalCalendarsResource<C> {
    type UriComponents = ();
    type MemberType = CalendarResource<C>;
    type PropType = PrincipalProp;

    fn get_path(&self) -> &str {
        &self.path
    }

    async fn get_members(&self) -> Result<Vec<Self::MemberType>> {
        let calendars = self
            .cal_store
            .read()
            .await
            .get_calendars(&self.principal)
            .await?;
        let mut out = Vec::new();
        for calendar in calendars {
            let path = format!("{}/{}", &self.path, &calendar.id);
            out.push(CalendarResource {
                cal_store: self.cal_store.clone(),
                calendar,
                path,
                prefix: self.prefix.clone(),
                principal: self.principal.clone(),
            })
        }
        Ok(out)
    }

    async fn acquire_from_request(
        req: HttpRequest,
        auth_info: AuthInfo,
        _uri_components: Self::UriComponents,
        prefix: String,
    ) -> Result<Self> {
        let cal_store = req
            .app_data::<Data<RwLock<C>>>()
            .ok_or(anyhow!("no calendar store in app_data!"))?
            .clone()
            .into_inner();
        Ok(Self {
            cal_store,
            prefix,
            principal: auth_info.user_id,
            path: req.path().to_string(),
        })
    }

    fn write_prop<W: std::io::Write>(
        &self,
        writer: &mut quick_xml::Writer<W>,
        prop: Self::PropType,
    ) -> Result<()> {
        match prop {
            PrincipalProp::Resourcetype => {
                write_resourcetype(writer, vec!["principal", "collection"])?
            }
            PrincipalProp::CurrentUserPrincipal | PrincipalProp::PrincipalUrl => {
                write_href_prop(
                    writer,
                    prop.into(),
                    &format!("{}/{}/", self.prefix, self.principal),
                )?;
            }
            PrincipalProp::CalendarHomeSet | PrincipalProp::CalendarUserAddressSet => {
                let propname: &'static str = prop.into();
                writer
                    .create_element(&format!("C:{propname}"))
                    .write_inner_content(|writer| {
                        writer
                            .create_element("href")
                            .write_text_content(BytesText::new(&format!(
                                "{}/{}/",
                                self.prefix, self.principal
                            )))?;
                        Ok::<(), quick_xml::Error>(())
                    })?;
            }
        };
        Ok(())
    }
}
