use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::{ContentType, Header};
use rocket::{Request, Response};

pub struct CacheControlFairing;

#[rocket::async_trait]
impl Fairing for CacheControlFairing {
    fn info(&self) -> Info {
        Info {
            name: "Cache-Control Header Setter",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _req: &'r Request<'_>, res: &mut Response<'r>) {
        if Some(ContentType::WOFF2) == res.content_type() {
            res.set_header(Header::new("Cache-Control", "max-age=31536000, public"));
        }
        if Some(ContentType::JavaScript) == res.content_type() {
            res.set_header(Header::new("Cache-Control", "max-age=600, public"));
        }
        if Some(ContentType::CSS) == res.content_type() {
            res.set_header(Header::new("Cache-Control", "max-age=600, public"));
        }
    }
}
