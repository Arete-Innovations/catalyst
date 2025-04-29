use rocket::{
    data::Data,
    request::{self, FromRequest, Request},
    route::{Handler, Outcome},
    Route,
};
use std::marker::PhantomData;

pub struct Guarded<G> {
    inner: Box<dyn Handler>,
    _marker: PhantomData<fn() -> G>,
}

impl<G> Guarded<G> {
    #[inline]
    pub fn new(inner: Box<dyn Handler>) -> Self {
        Self { inner, _marker: PhantomData }
    }
}

impl<G> Clone for Guarded<G> {
    fn clone(&self) -> Self {
        Self::new(self.inner.clone())
    }
}

#[rocket::async_trait]
impl<G> Handler for Guarded<G>
where
    for<'a> G: FromRequest<'a> + Send + Sync + 'static,
{
    async fn handle<'r>(&self, req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r> {
        match req.guard::<G>().await {
            request::Outcome::Error((status, _)) => return Outcome::error(status),
            request::Outcome::Forward(status) => return Outcome::forward(data, status),
            request::Outcome::Success(_) => (),
        }

        self.inner.handle(req, data).await
    }
}

pub fn with_guard<G>(routes: Vec<Route>) -> Vec<Route>
where
    for<'a> G: FromRequest<'a> + Send + Sync + 'static,
{
    routes
        .into_iter()
        .map(|mut r| {
            let original = r.handler.clone();
            r.handler = Box::new(Guarded::<G>::new(original));
            r
        })
        .collect()
}
