use std::io::Cursor;

use rocket::{
    http::{ContentType, Header, Status},
    request::Request,
    response::{self, Responder, Response},
};

use crate::meltdown::*;

pub struct HtmxSuccess;
pub struct HtmxError;
pub struct HtmxWarning;
pub struct HtmxInfo;

pub enum MessageType {
    Success,
    Error,
    Warning,
    Info,
}

pub struct Htmx {
    status: Status,
    html: String,
    message_type: MessageType,
    headers: Vec<Header<'static>>,
}

impl Htmx {
    pub fn success() -> HtmxSuccess {
        HtmxSuccess
    }

    pub fn error() -> HtmxError {
        HtmxError
    }

    pub fn warning() -> HtmxWarning {
        HtmxWarning
    }

    pub fn info() -> HtmxInfo {
        HtmxInfo
    }

    pub fn with_header(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.headers.push(Header::new(name, value.into()));
        self
    }

    pub fn with_redirect(self, url: impl Into<String>) -> Self {
        self.with_header("HX-Redirect", url)
    }

    pub fn with_location(self, url: impl Into<String>) -> Self {
        self.with_header("HX-Location", url)
    }
}

impl HtmxSuccess {
    pub fn with_content(content: impl Into<String>) -> Htmx {
        Htmx {
            status: Status::Ok,
            html: content.into(),
            message_type: MessageType::Success,
            headers: vec![Header::new("HX-Response-Type", "content"), Header::new("HX-Message-Type", "success")],
        }
    }

    pub fn with_notification(message: impl Into<String>) -> Htmx {
        Htmx {
            status: Status::Ok,
            html: message.into(),
            message_type: MessageType::Success,
            headers: vec![Header::new("HX-Response-Type", "notification"), Header::new("HX-Message-Type", "success")],
        }
    }
}

impl HtmxError {
    pub fn with_content(status: Status, content: impl Into<String>) -> Htmx {
        Htmx {
            status,
            html: content.into(),
            message_type: MessageType::Error,
            headers: vec![Header::new("HX-Response-Type", "content"), Header::new("HX-Message-Type", "error")],
        }
    }

    pub fn with_notification(status: Status, message: impl Into<String>) -> Htmx {
        Htmx {
            status,
            html: message.into(),
            message_type: MessageType::Error,
            headers: vec![Header::new("HX-Response-Type", "notification"), Header::new("HX-Message-Type", "error")],
        }
    }
}

impl HtmxWarning {
    pub fn with_content(content: impl Into<String>) -> Htmx {
        Htmx {
            status: Status::Ok,
            html: content.into(),
            message_type: MessageType::Warning,
            headers: vec![Header::new("HX-Response-Type", "content"), Header::new("HX-Message-Type", "warning")],
        }
    }

    pub fn with_notification(message: impl Into<String>) -> Htmx {
        Htmx {
            status: Status::Ok,
            html: message.into(),
            message_type: MessageType::Warning,
            headers: vec![Header::new("HX-Response-Type", "notification"), Header::new("HX-Message-Type", "warning")],
        }
    }
}

impl HtmxInfo {
    pub fn with_content(content: impl Into<String>) -> Htmx {
        Htmx {
            status: Status::Ok,
            html: content.into(),
            message_type: MessageType::Info,
            headers: vec![Header::new("HX-Response-Type", "content"), Header::new("HX-Message-Type", "info")],
        }
    }

    pub fn with_notification(message: impl Into<String>) -> Htmx {
        Htmx {
            status: Status::Ok,
            html: message.into(),
            message_type: MessageType::Info,
            headers: vec![Header::new("HX-Response-Type", "notification"), Header::new("HX-Message-Type", "info")],
        }
    }
}

impl<'r> Responder<'r, 'static> for Htmx {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        let html = self.html;
        let html_len = html.len();
        let status = self.status;
        let headers = self.headers;

        let mut builder = Response::build();
        builder.status(status);
        builder.header(ContentType::HTML);
        builder.sized_body(html_len, Cursor::new(html));

        for header in headers {
            builder.header(header);
        }

        builder.ok()
    }
}

pub type HtmxResult = Result<Htmx, Htmx>;

pub trait IntoHtmx<T> {
    fn into_htmx(self) -> HtmxResult;
}

impl<T> IntoHtmx<T> for Result<T, MeltDown> {
    fn into_htmx(self) -> HtmxResult {
        match self {
            Ok(_) => Err(HtmxError::with_notification(
                Status::InternalServerError,
                "Cannot convert success value to HTMX. Use map() to provide HTML content.",
            )),
            Err(error) => Err(HtmxError::with_notification(error.status_code(), error.user_message())),
        }
    }
}
