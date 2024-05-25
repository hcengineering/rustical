use actix_web::http::Method;
use actix_web::web::{self, Data};
use actix_web::{guard, HttpResponse, Responder};
use resources::calendar::CalendarResource;
use resources::event::EventResource;
use resources::principal::PrincipalResource;
use resources::root::RootResource;
use routes::{calendar, event};
use rustical_auth::CheckAuthentication;
use rustical_dav::error::Error;
use rustical_dav::propfind::{handle_propfind, ServicePrefix};
use rustical_store::calendar::CalendarStore;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod resources;
pub mod routes;

pub struct CalDavContext<C: CalendarStore + ?Sized> {
    pub store: Arc<RwLock<C>>,
}

pub fn configure_well_known(cfg: &mut web::ServiceConfig, caldav_root: String) {
    cfg.service(web::redirect("/caldav", caldav_root).permanent());
}

pub fn configure_dav<A: CheckAuthentication, C: CalendarStore + ?Sized>(
    cfg: &mut web::ServiceConfig,
    prefix: String,
    auth: Arc<A>,
    store: Arc<RwLock<C>>,
) {
    let propfind_method = || web::method(Method::from_str("PROPFIND").unwrap());
    let report_method = || web::method(Method::from_str("REPORT").unwrap());
    let mkcol_method = || web::method(Method::from_str("MKCOL").unwrap());

    cfg.app_data(Data::new(CalDavContext {
        store: store.clone(),
    }))
    .app_data(Data::new(ServicePrefix(prefix)))
    .app_data(Data::from(store.clone()))
    .app_data(Data::from(auth))
    .service(
        web::resource("{path:.*}")
            // Without the guard this service would handle all requests
            .guard(guard::Method(Method::OPTIONS))
            .to(options_handler),
    )
    .service(web::resource("").route(propfind_method().to(handle_propfind::<A, RootResource>)))
    .service(
        web::resource("/{principal}")
            .route(propfind_method().to(handle_propfind::<A, PrincipalResource<C>>)),
    )
    // .service(DavResourceService::<PrincipalResource>::new("/{principal}"))
    .service(
        web::resource("/{principal}/{calendar}")
            .route(report_method().to(calendar::route_report_calendar::<A, C>))
            // .route(web::method(propfind_method()).to(route_propfind::<A, CalendarResource<C>, C>))
            .route(propfind_method().to(handle_propfind::<A, CalendarResource<C>>))
            .route(mkcol_method().to(calendar::route_mkcol_calendar::<A, C>))
            .route(web::method(Method::DELETE).to(calendar::delete_calendar::<A, C>)),
    )
    // .service(web::resource("/{principal}/{calendar}").route(route))
    .service(
        web::resource("/{principal}/{calendar}/{event}")
            // .route(web::method(propfind_method()).to(route_propfind::<A, EventResource<C>, C>))
            .route(propfind_method().to(handle_propfind::<A, EventResource<C>>))
            .route(web::method(Method::DELETE).to(event::delete_event::<A, C>))
            .route(web::method(Method::GET).to(event::get_event::<A, C>))
            .route(web::method(Method::PUT).to(event::put_event::<A, C>)),
    );
}

async fn options_handler() -> impl Responder {
    HttpResponse::Ok()
        .insert_header((
            "Allow",
            "OPTIONS, GET, HEAD, POST, PUT, REPORT, PROPFIND, PROPPATCH, MKCOL",
        ))
        .insert_header((
            "DAV",
            "1, 2, 3, calendar-access, extended-mkcol",
            // "1, 2, 3, calendar-access, addressbook, extended-mkcol",
        ))
        .body("options")
}
