use actix_web::body::MessageBody;
use actix_web::dev::{ServiceFactory, ServiceRequest, ServiceResponse};
use actix_web::middleware::NormalizePath;
use actix_web::{App, web};
use rustical_caldav::caldav_service;
//use rustical_carddav::carddav_service;
use rustical_frontend::nextcloud_login::{NextcloudFlows, configure_nextcloud_login};
use rustical_frontend::{FrontendConfig, configure_frontend};
use rustical_oidc::OidcConfig;
use rustical_store::auth::AuthenticationProvider;
use rustical_store::{AddressbookStore, CalendarStore, SubscriptionStore};
use std::sync::Arc;
use tracing_actix_web::TracingLogger;

use crate::config::NextcloudLoginConfig;

#[allow(clippy::too_many_arguments)]
pub fn make_app<AS: AddressbookStore, CS: CalendarStore, S: SubscriptionStore>(
    addr_store: Arc<AS>,
    cal_store: Arc<CS>,
    subscription_store: Arc<S>,
    auth_provider: Arc<impl AuthenticationProvider>,
    frontend_config: FrontendConfig,
    oidc_config: Option<OidcConfig>,
    nextcloud_login_config: NextcloudLoginConfig,
    nextcloud_flows_state: Arc<NextcloudFlows>,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<impl MessageBody>,
        Config = (),
        InitError = (),
        Error = actix_web::Error,
    >,
> {
    let mut app = App::new()
        // .wrap(Logger::new("[%s] %r"))
        .wrap(TracingLogger::default())
        .wrap(NormalizePath::trim())
        .service(web::scope("/caldav").service(caldav_service(
            auth_provider.clone(),
            cal_store.clone(),
            addr_store.clone(),
            subscription_store.clone(),
        )))
        /*
        .service(web::scope("/carddav").service(carddav_service(
            auth_provider.clone(),
            addr_store.clone(),
            subscription_store,
        )))
        */
        .service(
            web::scope("/.well-known").service(web::redirect("/caldav", "/caldav")), //.service(web::redirect("/carddav", "/carddav")),
        );

    if nextcloud_login_config.enabled {
        app = app.configure(|cfg| {
            configure_nextcloud_login(
                cfg,
                nextcloud_flows_state,
                auth_provider.clone(),
                frontend_config.secret_key,
            )
        });
    }
    if frontend_config.enabled {
        app = app
            .service(web::scope("/frontend").configure(|cfg| {
                configure_frontend(
                    cfg,
                    auth_provider.clone(),
                    cal_store.clone(),
                    addr_store.clone(),
                    frontend_config,
                    oidc_config,
                )
            }))
            .service(web::redirect("/", "/frontend").see_other());
    }
    app
}
