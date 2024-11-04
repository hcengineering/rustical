use crate::{
    extension::ResourceExtension,
    privileges::UserPrivilegeSet,
    resource::{InvalidProperty, Resource},
    xml::{HrefElement, Resourcetype},
};
use actix_web::dev::ResourceMap;
use rustical_store::auth::User;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use strum::{EnumString, VariantNames};

#[derive(Clone)]
pub struct CommonPropertiesExtension<R: Resource>(PhantomData<R>);

impl<R: Resource> Default for CommonPropertiesExtension<R> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[derive(Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum CommonPropertiesProp {
    // WebDAV (RFC 2518)
    #[serde(skip_deserializing)]
    Resourcetype(Resourcetype),

    // WebDAV Current Principal Extension (RFC 5397)
    CurrentUserPrincipal(HrefElement),

    // WebDAV Access Control Protocol (RFC 3477)
    CurrentUserPrivilegeSet(UserPrivilegeSet),
    Owner(Option<HrefElement>),

    #[serde(other)]
    Invalid,
}

impl InvalidProperty for CommonPropertiesProp {
    fn invalid_property(&self) -> bool {
        matches!(self, Self::Invalid)
    }
}

#[derive(EnumString, VariantNames, Clone)]
#[strum(serialize_all = "kebab-case")]
pub enum CommonPropertiesPropName {
    Resourcetype,
    CurrentUserPrincipal,
    CurrentUserPrivilegeSet,
    Owner,
}

impl<R: Resource> ResourceExtension<R> for CommonPropertiesExtension<R>
where
    R::Prop: From<CommonPropertiesProp>,
{
    type Prop = CommonPropertiesProp;
    type PropName = CommonPropertiesPropName;
    type Error = R::Error;

    fn get_prop(
        &self,
        resource: &R,
        rmap: &ResourceMap,
        user: &User,
        prop: Self::PropName,
    ) -> Result<Self::Prop, Self::Error> {
        Ok(match prop {
            CommonPropertiesPropName::Resourcetype => {
                CommonPropertiesProp::Resourcetype(Resourcetype(R::get_resourcetype()))
            }
            CommonPropertiesPropName::CurrentUserPrincipal => {
                CommonPropertiesProp::CurrentUserPrincipal(
                    R::PrincipalResource::get_url(rmap, [&user.id])
                        .unwrap()
                        .into(),
                )
            }
            CommonPropertiesPropName::CurrentUserPrivilegeSet => {
                CommonPropertiesProp::CurrentUserPrivilegeSet(resource.get_user_privileges(user)?)
            }
            CommonPropertiesPropName::Owner => CommonPropertiesProp::Owner(
                resource
                    .get_owner()
                    .map(|owner| R::PrincipalResource::get_url(rmap, [owner]).unwrap().into()),
            ),
        })
    }
}
