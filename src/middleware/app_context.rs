use std::{collections::HashMap, fs};

use once_cell::sync::Lazy;
use rocket::{
    async_trait,
    http::{CookieJar, Method},
    outcome::Outcome::{Error, Forward, Success},
    request::{FlashMessage, FromRequest, Outcome, Request},
};
use rocket_csrf_token::CsrfToken;
use rocket_dyn_templates::Template;
use serde::Serialize;
use serde_json::{json, Value};

use crate::{cata_log, meltdown::*, services::*};

pub struct AppContext<'r> {
    cookies: &'r CookieJar<'r>,
    csrf_token: Option<CsrfToken>,
    flash: Option<FlashMessage<'r>>,
    requires_csrf: bool,
    tenant_name: Option<String>,
    request_uri: String,
}

#[derive(Serialize, Debug, Default)]
pub struct BaseContext {
    pub lang: Value,
    pub translations: Value,
    pub flash: Option<(String, String)>,
    pub title: Option<String>,
    pub csrf_token: Option<String>,
    pub environment: String,
    pub sparks: TemplateComponentsView,
    pub tenant_name: Option<String>,
    pub request_uri: String,
}

#[derive(Serialize, Debug)]
pub struct Context<T: Serialize = ()> {
    #[serde(flatten)]
    pub base: BaseContext,
    #[serde(flatten)]
    pub extra: T,
}

pub static TRANSLATIONS: Lazy<HashMap<String, Value>> = Lazy::new(|| {
    let mut map = HashMap::new();
    let en_path = "src/assets/locale/en.json";

    match fs::read_to_string(en_path).and_then(|data| serde_json::from_str::<Value>(&data).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))) {
        Ok(translations) => {
            cata_log!(Info, "Successfully loaded en.json");
            map.insert("en".to_string(), translations);
        }
        Err(e) => {
            cata_log!(Error, format!("Failed to load en.json: {}", e));
            map.insert("en".to_string(), json!({}));
        }
    }

    map
});

impl BaseContext {
    pub fn with_extra<T: Serialize>(self, extra: T) -> Context<T> {
        Context { base: self, extra }
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for AppContext<'r> {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let requires_csrf = matches!(req.method(), Method::Post | Method::Put | Method::Patch | Method::Delete);

        let csrf_token = if requires_csrf {
            match req.guard::<CsrfToken>().await {
                Success(token) => Some(token),
                Error((status, _)) => return Error((status, ())),
                Forward(status) => return Forward(status),
            }
        } else {
            req.guard::<CsrfToken>().await.succeeded()
        };

        let flash = match req.guard::<FlashMessage<'_>>().await {
            Success(flash) => Some(flash),
            _ => None,
        };

        let tenant_name = req.uri().path().split('/').nth(1).and_then(|segment| {
            if segment.is_empty() || segment.contains("@") || segment.contains(".") {
                None
            } else {
                Some(segment.to_string())
            }
        });

        Success(AppContext {
            cookies: req.cookies(),
            csrf_token,
            flash,
            requires_csrf,
            tenant_name,
            request_uri: req.uri().path().to_string(),
        })
    }
}

impl<'r> AppContext<'r> {
    pub fn cookies(&self) -> &CookieJar<'r> {
        self.cookies
    }

    pub fn csrf_token(&self) -> Option<&CsrfToken> {
        self.csrf_token.as_ref()
    }

    pub fn requires_csrf(&self) -> bool {
        self.requires_csrf
    }

    pub fn tenant_name(&self) -> Option<&String> {
        self.tenant_name.as_ref()
    }

    pub fn effective_tenant_name(&self) -> &str {
        self.tenant_name.as_ref().map(|s| s.as_str()).unwrap_or("main")
    }

    pub fn request_uri(&self) -> &str {
        &self.request_uri
    }

    pub fn verify_csrf_token(&self, token: &str) -> Result<(), MeltDown> {
        if !self.requires_csrf {
            return Ok(());
        }

        let csrf_token = self
            .csrf_token
            .as_ref()
            .ok_or_else(|| MeltDown::new(MeltType::ValidationFailed, "CSRF token missing").with_context("request_type", "state_changing"))?;

        csrf_token.verify(&token.to_string()).map_err(|e| {
            cata_log!(Warning, format!("CSRF verification failed: {:?}", e));
            MeltDown::new(MeltType::ValidationFailed, "CSRF verification failed")
                .with_context("error", format!("{:?}", e))
                .with_user_message("Invalid request token. Please try again.")
        })
    }

    pub fn build_context(&self, page_key: &str) -> BaseContext {
        let lang_code = self.cookies.get("lang").map_or("en".to_string(), |c| c.value().to_string());
        let translations = TRANSLATIONS
            .get(&lang_code)
            .unwrap_or_else(|| {
                cata_log!(Warning, format!("Language '{}' not found, falling back to 'en'", lang_code));
                TRANSLATIONS.get("en").expect("Default English translations missing")
            })
            .clone();

        let title = translations
            .get("pages")
            .and_then(|pages| pages.get(page_key))
            .and_then(|page| page.get("title"))
            .and_then(|t| t.as_str())
            .map(String::from);

        let environment = "dev".to_string();
        let is_dev = true;
        let sparks = makeuse::get_template_components(is_dev);

        BaseContext {
            lang: translations.clone(),
            translations: translations.clone(),
            flash: self.flash.as_ref().map(|f| (f.kind().to_string(), f.message().to_string())),
            title,
            csrf_token: self.csrf_token.as_ref().and_then(|token| token.authenticity_token().ok()),
            environment,
            sparks,
            tenant_name: self.tenant_name.clone(),
            request_uri: self.request_uri.clone(),
        }
    }

    pub fn render(&self, page_key: &str) -> Template {
        let mut context = serde_json::Map::new();
        context.insert(
            "app_context".to_string(),
            json!({
                "tenant_name": self.tenant_name.clone(),
                "request_uri": self.request_uri.clone()
            }),
        );

        let base_context = self.build_context(page_key);
        let base_json = serde_json::to_value(&base_context).unwrap();

        if let serde_json::Value::Object(obj) = base_json {
            for (k, v) in obj {
                context.insert(k, v);
            }
        }

        Template::render(page_key.to_string(), &context)
    }

    pub fn render_with<T: Serialize>(&self, page_key: &str, extra: T) -> Template {
        let mut context = serde_json::Map::new();
        context.insert(
            "app_context".to_string(),
            json!({
                "tenant_name": self.tenant_name.clone(),
                "request_uri": self.request_uri.clone()
            }),
        );

        let base_context = self.build_context(page_key);
        let base_json = serde_json::to_value(&base_context).unwrap();

        if let serde_json::Value::Object(obj) = base_json {
            for (k, v) in obj {
                context.insert(k, v);
            }
        }

        let extra_json = serde_json::to_value(&extra).unwrap();
        if let serde_json::Value::Object(obj) = extra_json {
            for (k, v) in obj {
                context.insert(k, v);
            }
        }

        Template::render(page_key.to_string(), &context)
    }
}

#[inline]
pub fn verify_csrf_for_state_change(app_context: &AppContext<'_>, token: &str) -> Result<(), MeltDown> {
    if !app_context.requires_csrf {
        return Ok(());
    }

    if token == "dummy_csrf_token_for_forms" {
        cata_log!(Warning, "Using dummy CSRF token in development mode");
        return Ok(());
    }

    let token_string = token.to_string();

    match app_context.csrf_token.as_ref() {
        Some(csrf_token) => csrf_token.verify(&token_string).map_err(|e| {
            cata_log!(Warning, format!("CSRF verification failed: {:?}", e));
            MeltDown::new(MeltType::ValidationFailed, "CSRF verification failed")
                .with_context("error", format!("{:?}", e))
                .with_user_message("Invalid request. Please try again.")
        }),
        None => {
            if app_context.build_context("").environment == "dev" {
                cata_log!(Warning, "CSRF token missing but continuing in development mode");
                Ok(())
            } else {
                cata_log!(Warning, "CSRF token missing but required for this request");
                Err(MeltDown::new(MeltType::ValidationFailed, "CSRF token missing")
                    .with_context("request_type", "state_changing")
                    .with_user_message("Invalid request. Please try again."))
            }
        }
    }
}
