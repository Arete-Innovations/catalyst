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
    let auth_routes = public::auth::routes();
    let admin_routes = private::admin::routes();
    let user_routes = private::user::routes();
    let home_routes = public::home::routes();

    let all_routes = auth_routes
        .into_iter()
        .chain(admin_routes.into_iter())
        .chain(user_routes.into_iter())
        .chain(home_routes.into_iter())
        .collect::<Vec<_>>();

    rocket::build()
        .mount("/", all_routes)
        .mount("/public", FileServer::from(relative!("public")))
        .register("/", catchers![unauthorized, not_found])
        .attach(Template::fairing())
        .attach(CacheControlFairing)
        .attach(rocket_csrf_token::Fairing::default())
        .attach(sparks::SparkLoggingFairing)
        .all_sparks()
        .attach(AdHoc::on_liftoff("Start Scheduler", |_rocket| {
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
