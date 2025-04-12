use crate::middleware::{AdminGuard, ApiKeyGuard, UserGuard};
use rocket::route::Route;
use rocket::Build;
use rocket::Rocket;
use std::marker::PhantomData;

pub struct RouteGroupWithGuard<G>
where
    G: Send + Sync + 'static,
{
    prefix: String,
    routes: Vec<Route>,
    _guard: PhantomData<G>,
    content_type: Option<String>,
}

impl<G> RouteGroupWithGuard<G>
where
    G: Send + Sync + 'static,
{
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
            routes: Vec::new(),
            _guard: PhantomData,
            content_type: None,
        }
    }

    pub fn with_guard(routes: Vec<Route>) -> Self {
        Self {
            prefix: "/".to_string(),
            routes,
            _guard: PhantomData,
            content_type: None,
        }
    }

    pub fn with_content_type(mut self, content_type: &str) -> Self {
        self.content_type = Some(content_type.to_string());
        self
    }

    pub fn add_routes(mut self, routes: Vec<Route>) -> Self {
        self.routes.extend(routes);
        self
    }

    pub fn attach_to(self, rocket: Rocket<Build>) -> Rocket<Build> {
        rocket.mount(self.prefix, self.routes)
    }
}

pub trait RocketExt {
    fn attach_admin_guard(self, routes: Vec<Route>) -> Self;
    fn attach_user_guard(self, routes: Vec<Route>) -> Self;
    fn attach_api_guard(self, routes: Vec<Route>) -> Self;

    fn attach_html_routes(self, prefix: &str, routes: Vec<Route>) -> Self;
    fn attach_json_routes(self, prefix: &str, routes: Vec<Route>) -> Self;

    fn attach_admin_html_routes(self, prefix: &str, routes: Vec<Route>) -> Self;
    fn attach_user_html_routes(self, prefix: &str, routes: Vec<Route>) -> Self;
    fn attach_admin_json_routes(self, prefix: &str, routes: Vec<Route>) -> Self;
    fn attach_user_json_routes(self, prefix: &str, routes: Vec<Route>) -> Self;
}

impl RocketExt for Rocket<Build> {
    fn attach_admin_guard(self, routes: Vec<Route>) -> Self {
        RouteGroupWithGuard::<AdminGuard>::with_guard(routes).attach_to(self)
    }

    fn attach_user_guard(self, routes: Vec<Route>) -> Self {
        RouteGroupWithGuard::<UserGuard>::with_guard(routes).attach_to(self)
    }

    fn attach_api_guard(self, routes: Vec<Route>) -> Self {
        RouteGroupWithGuard::<ApiKeyGuard>::with_guard(routes).attach_to(self)
    }

    fn attach_html_routes(self, prefix: &str, routes: Vec<Route>) -> Self {
        let group = RouteGroupWithGuard::<()>::new(prefix).with_content_type("text/html").add_routes(routes);
        group.attach_to(self)
    }

    fn attach_json_routes(self, prefix: &str, routes: Vec<Route>) -> Self {
        let group = RouteGroupWithGuard::<()>::new(prefix).with_content_type("application/json").add_routes(routes);
        group.attach_to(self)
    }

    fn attach_admin_html_routes(self, prefix: &str, routes: Vec<Route>) -> Self {
        let group = RouteGroupWithGuard::<AdminGuard>::new(prefix).with_content_type("text/html").add_routes(routes);
        group.attach_to(self)
    }

    fn attach_user_html_routes(self, prefix: &str, routes: Vec<Route>) -> Self {
        let group = RouteGroupWithGuard::<UserGuard>::new(prefix).with_content_type("text/html").add_routes(routes);
        group.attach_to(self)
    }

    fn attach_admin_json_routes(self, prefix: &str, routes: Vec<Route>) -> Self {
        let group = RouteGroupWithGuard::<AdminGuard>::new(prefix).with_content_type("application/json").add_routes(routes);
        group.attach_to(self)
    }

    fn attach_user_json_routes(self, prefix: &str, routes: Vec<Route>) -> Self {
        let group = RouteGroupWithGuard::<UserGuard>::new(prefix).with_content_type("application/json").add_routes(routes);
        group.attach_to(self)
    }
}
