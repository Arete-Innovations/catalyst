use crate::routes::*;
use rocket::catch;
use rocket::response::Redirect;
use rocket::uri;

#[catch(401)]
pub fn unauthorized() -> Redirect {
    Redirect::to(uri!(public::auth::get_login))
}

#[catch(404)]
pub fn not_found() -> Redirect {
    Redirect::to(uri!(public::home::page_not_found))
}
