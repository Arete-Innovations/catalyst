#[macro_use]
extern crate rocket;

use rocket::fairing::AdHoc;
use rocket::fs::{relative, FileServer};
use rocket_dyn_templates::Template;
use tokio::spawn;

mod bootstrap;
mod database;
mod meltdown;
mod middleware;
mod models;
mod routes;
mod services;
mod structs;

use bootstrap::*;
use middleware::*;
use routes::*;
use services::*;

#[launch]
fn rocket() -> _ {
    bootstrap();
    cata_log!(Info, "Starting server...");
    rocket::build()
        .mount("/", home::routes())
        .attach_admin_guard(admin::admin_routes())
        .attach_user_guard(user::user_routes())
        .attach_user_guard(api::user_partials::user_partial_routes())
        .attach_api_guard(api::v1::api_v1_routes())
        .attach_admin_guard(api::admin_partials::admin_partial_routes())
        .mount("/public", FileServer::from(relative!("public")))
        .register("/", catchers![unauthorized, forbidden, not_found, internal_error, unprocessable_entity])
        .attach(Template::fairing())
        .attach(CacheControlFairing)
        .attach(rocket_csrf_token::Fairing::default())
        .attach(api_logger::ApiLogFairing)
        .attach(sparks::SparkLoggingFairing)
        .all_sparks()
        .attach(AdHoc::on_liftoff("Cronjob Scheduler", |_rocket| {
            Box::pin(async move {
                spawn(scheduler());
                cata_log!(Info, "Scheduler has started.");
            })
        }))
        .attach(AdHoc::on_response("Template Error", |_, res| {
            Box::pin(async move {
                if res.status().code >= 400 {
                    cata_log!(Error, format!("Template error: {}", res.status()));
                }
            })
        }))
        .attach(Gzip)
}
