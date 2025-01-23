use actix_web::dev::{AppService, HttpServiceFactory};
use actix_web::error::UrlGenerationError;
use actix_web::test::TestRequest;
use actix_web::web::Data;
use actix_web::{dev::ResourceMap, http::Method, web, ResponseError};
use async_trait::async_trait;
use rustical_store::auth::User;
use serde::Deserialize;
use std::str::FromStr;

use super::methods::{route_delete, route_propfind, route_proppatch};
use super::Resource;

#[async_trait(?Send)]
pub trait ResourceService: Sized + 'static {
    type MemberType: Resource<Error = Self::Error>;
    type PathComponents: for<'de> Deserialize<'de> + Sized + Clone + 'static; // defines how the resource URI maps to parameters, i.e. /{principal}/{calendar} -> (String, String)
    type Resource: Resource<Error = Self::Error>;
    type Error: ResponseError + From<crate::Error>;

    async fn get_members(
        &self,
        _user: &User,
        _path_components: &Self::PathComponents,
    ) -> Result<Vec<(String, Self::MemberType)>, Self::Error> {
        Ok(vec![])
    }

    async fn get_resource(
        &self,
        _user: &User,
        _path: &Self::PathComponents,
    ) -> Result<Self::Resource, Self::Error>;
    async fn save_resource(
        &self,
        _user: &User,
        _path: &Self::PathComponents,
        _file: Self::Resource,
    ) -> Result<(), Self::Error> {
        Err(crate::Error::Unauthorized.into())
    }
    async fn delete_resource(
        &self,
        _user: &User,
        _path: &Self::PathComponents,
        _use_trashbin: bool,
    ) -> Result<(), Self::Error> {
        Err(crate::Error::Unauthorized.into())
    }

    #[inline]
    fn actix_resource(self) -> actix_web::Resource {
        Self::actix_additional_routes(
            web::resource("")
                .app_data(Data::new(self))
                .route(
                    web::method(Method::from_str("PROPFIND").unwrap()).to(route_propfind::<Self>),
                )
                .route(
                    web::method(Method::from_str("PROPPATCH").unwrap()).to(route_proppatch::<Self>),
                )
                .delete(route_delete::<Self>),
        )
    }

    /// Hook for other resources to insert their additional methods (i.e. REPORT, MKCALENDAR)
    #[inline]
    fn actix_additional_routes(res: actix_web::Resource) -> actix_web::Resource {
        res
    }
}

pub trait NamedRoute {
    fn route_name() -> &'static str;

    fn get_url<U, I>(rmap: &ResourceMap, elements: U) -> Result<String, UrlGenerationError>
    where
        U: IntoIterator<Item = I>,
        I: AsRef<str>,
    {
        Ok(rmap
            .url_for(
                &TestRequest::default().to_http_request(),
                Self::route_name(),
                elements,
            )?
            .path()
            .to_owned())
    }
}

pub struct ResourceServiceRoute<RS: ResourceService>(pub RS);

impl<RS: ResourceService> HttpServiceFactory for ResourceServiceRoute<RS> {
    fn register(self, config: &mut AppService) {
        self.0.actix_resource().register(config);
    }
}
