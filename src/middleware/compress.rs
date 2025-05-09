use std::io::{Cursor, Write};

use flate2::{write::GzEncoder, Compression};
use rocket::{
    fairing::{Fairing, Info, Kind},
    http::Header,
    Request, Response,
};

pub struct Gzip;

#[rocket::async_trait]
impl Fairing for Gzip {
    fn info(&self) -> Info {
        Info {
            name: "Gzip Compression",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _req: &'r Request<'_>, res: &mut Response<'r>) {
        if let Some(body) = res.body_mut().take().to_bytes().await.ok() {
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&body).unwrap();
            let compressed_body = encoder.finish().unwrap();

            res.set_header(Header::new("Content-Encoding", "gzip"));
            res.set_sized_body(compressed_body.len(), Cursor::new(compressed_body));
        }
    }
}
