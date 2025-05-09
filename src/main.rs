#[macro_use]
extern crate rocket;

use rocket::{
    fairing::AdHoc,
    fs::{relative, FileServer},
};
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
async fn rocket() -> _ {
    bootstrap().await;
    cata_log!(Info, "Starting server...");
    let mut rocket_app = rocket::build()
        .mount("/", home::routes())
        .mount("/", with_guard::<AdminGuard>(admin_routes()))
        .mount("/", with_guard::<AdminGuard>(admin_partial_routes()))
        .mount("/", with_guard::<UserGuard>(user_routes()))
        .mount("/", with_guard::<UserGuard>(user_partial_routes()))
        .mount("/", with_guard::<ApiKeyGuard>(api_v1_routes()))
        .mount("/public", FileServer::from(relative!("public")))
        .register("/", catchers![unauthorized, forbidden, not_found, internal_error, unprocessable_entity])
        .attach(Template::fairing())
        .attach(rocket_csrf_token::Fairing::default())
        .attach(api_logger::ApiLogFairing)
        .attach(sparks::SparkLoggingFairing)
        .all_sparks();

    if let Some(config) = APP_CONFIG.get() {
        if config.settings.environment == "prod" {
            cata_log!(Info, "Production mode: enabling response caching");
            rocket_app = rocket_app.attach(CacheControlFairing);
        }
    }

    rocket_app
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
