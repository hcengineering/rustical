use super::AuthenticationProvider;
use actix_session::Session;
use actix_web::{
    FromRequest, HttpMessage,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
    http::header::Header,
};
use actix_web_httpauth::headers::authorization::{Authorization, Basic};
use std::{
    future::{Future, Ready, ready},
    pin::Pin,
    sync::Arc,
};
use tracing::{Instrument, info_span};

pub struct AuthenticationMiddleware<AP: AuthenticationProvider> {
    auth_provider: Arc<AP>,
}

impl<AP: AuthenticationProvider> AuthenticationMiddleware<AP> {
    pub fn new(auth_provider: Arc<AP>) -> Self {
        Self { auth_provider }
    }
}

impl<AP: AuthenticationProvider, S, B> Transform<S, ServiceRequest> for AuthenticationMiddleware<AP>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    S::Future: 'static,
    B: 'static,
    AP: 'static,
{
    type Error = actix_web::Error;
    type Response = ServiceResponse<B>;
    type InitError = ();
    type Transform = InnerAuthenticationMiddleware<S, AP>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(InnerAuthenticationMiddleware {
            service: Arc::new(service),
            auth_provider: Arc::clone(&self.auth_provider),
        }))
    }
}

pub struct InnerAuthenticationMiddleware<S, AP: AuthenticationProvider> {
    service: Arc<S>,
    auth_provider: Arc<AP>,
}

impl<S, B, AP> Service<ServiceRequest> for InnerAuthenticationMiddleware<S, AP>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    S::Future: 'static,
    AP: AuthenticationProvider,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = Arc::clone(&self.service);
        let auth_provider = Arc::clone(&self.auth_provider);

        Box::pin(async move {
            if let Ok(auth) = Authorization::<Basic>::parse(req.request()) {
                // A simple stupid way to pass workspace into auth provider without modifications of interfaces
                let mut ws = None;
                let mut parts = req.request().path().split("/");
                let mut get_ws = false;
                while let Some(part) = parts.next() {
                    if part == "calendar" {
                        get_ws = true;
                    } else if get_ws {
                        ws = Some(part.to_string());
                        break;
                    }
                }
                let user_id = if let Some(ws) = ws {
                    &format!(
                        "{:?}|{}|{}",
                        req.request().connection_info().peer_addr(),
                        auth.as_ref().user_id(),
                        ws
                    )
                } else {
                    &format!(
                        "{:?}|{}",
                        req.request().connection_info().peer_addr(),
                        auth.as_ref().user_id()
                    )
                };
                //let user_id = auth.as_ref().user_id();
                if let Some(password) = auth.as_ref().password() {
                    if let Ok(Some(user)) = auth_provider
                        .validate_app_token(user_id, password)
                        .instrument(info_span!("validate_user_token"))
                        .await
                    {
                        req.extensions_mut().insert(user);
                    }
                }
            }

            // In macOS calendar user the Advanced mode when adding acount
            // and provide these test credentials and use server url
            // /caldav/principal/4b7c82f7-d3d4-43b6-ae10-e891fe1e0565
            // let test_user_id = "4b7c82f7-d3d4-43b6-ae10-e891fe1e0565";
            // let test_password = "kTY9gt5ez1uziC5vTgIDGzZi";
            // let mut ws = None;
            // let mut parts = req.request().path().split("/");
            // let mut get_ws = false;
            // while let Some(part) = parts.next() {
            //     if part == "calendar" {
            //         get_ws = true;
            //     } else if get_ws {
            //         ws = Some(part.to_string());
            //         break;
            //     }
            // }
            // let user_id = if let Some(ws) = ws {
            //     &format!(
            //         "{:?}|{}|{}",
            //         req.request().connection_info().peer_addr(),
            //         test_user_id,
            //         ws
            //     )
            // } else {
            //     &format!(
            //         "{:?}|{}",
            //         req.request().connection_info().peer_addr(),
            //         test_user_id
            //     )
            // };
            // if let Ok(Some(user)) = auth_provider
            //     .validate_app_token(user_id, test_password)
            //     .instrument(info_span!("validate_user_token"))
            //     .await
            // {
            //     req.extensions_mut().insert(user);
            // }

            // Extract user from session cookie
            if let Ok(session) = Session::extract(req.request()).await {
                match session.get::<String>("user") {
                    Ok(Some(user_id)) => match auth_provider.get_principal(&user_id).await {
                        Ok(Some(user)) => {
                            req.extensions_mut().insert(user);
                        }
                        Ok(None) => {}
                        Err(err) => {
                            dbg!(err);
                        }
                    },
                    Ok(None) => {}
                    Err(err) => {
                        dbg!(err);
                    }
                };
            }
            service.call(req).await
        })
    }
}
