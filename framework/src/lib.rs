use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;
use tide::http::mime;

pub use derive::Page;

pub mod prelude {
    pub use super::{Constructor, Error, FrameworkBuilder, Page, Request, Result, TableRow};
    pub use async_trait::async_trait;
}

pub struct FrameworkBuilder<T> {
    state: T,
    pages: Vec<Box<dyn PageHandlerTrait<T>>>,
}

impl Default for FrameworkBuilder<()> {
    fn default() -> Self {
        Self {
            state: (),
            pages: Vec::new(),
        }
    }
}

impl<T: Send + Sync + Clone + 'static> FrameworkBuilder<T> {
    pub fn with_state(state: T) -> Self {
        Self {
            state,
            pages: Vec::new(),
        }
    }

    pub fn add_page<P: Page<T> + Sync>(mut self) -> Self {
        self.pages.push(PageHandler::<T, P>::boxed());
        self
    }

    pub async fn run(self, listener: impl tide::listener::ToListener<T>) -> Result {
        let mut server = tide::with_state(self.state);
        server.with(FrameworkMiddleware { pages: self.pages });
        server
            .at("/static/script.js")
            .get(|_| serve_static_file(mime::JAVASCRIPT, include_str!("../static/script.js")));
        server.listen(listener).await.map_err(Error::Tide)
    }
}

async fn serve_static_file(mime: mime::Mime, str: &'static str) -> tide::Result<tide::Response> {
    Ok(tide::Response::builder(200)
        .content_type(mime)
        .body(str)
        .build())
}

struct FrameworkMiddleware<T> {
    pages: Vec<Box<dyn PageHandlerTrait<T>>>,
}

impl<T> FrameworkMiddleware<T> {
    fn generate_html(&self, status: u16, body: impl AsRef<str>) -> tide::Result {
        const HEADER: &str = r#"<!DOCTYPE html>
<html>
    <head>
        <script type='text/javascript' src='https://cdnjs.cloudflare.com/ajax/libs/knockout/3.5.0/knockout-min.js'></script>
        <script type='text/javascript' src='/static/script.js'></script>
    </head>
    <body>
"#;
        const FOOTER: &str = r#"    </body>
        </html>"#;
        let body = format!("{}{}{}", HEADER, body.as_ref(), FOOTER);
        Ok(tide::Response::builder(status)
            .content_type(mime::HTML)
            .body(body)
            .build())
    }

    fn generate_json(&self, status: u16, json: serde_json::Value) -> tide::Result {
        Ok(tide::Response::builder(status)
            .content_type(mime::JSON)
            .body(json.to_string())
            .build())
    }
}

#[async_trait]
impl<T> tide::Middleware<T> for FrameworkMiddleware<T>
where
    T: Clone + Send + Sync + 'static,
{
    async fn handle(&self, request: tide::Request<T>, next: tide::Next<'_, T>) -> tide::Result {
        if request.url().path().starts_with("/static") {
            return Ok(next.run(request).await);
        }
        let path = request.url().path();
        if let Some(stripped) = path.strip_suffix(".json") {
            for page in &self.pages {
                if stripped == page.url() {
                    let (state, body) = match page.generate_json(Request::new(request)).await {
                        Ok(response) => (200, response),
                        Err(e) => (500, serde_json::json!({ "error": format!("{:?}", e) })),
                    };
                    return self.generate_json(state, body);
                }
            }
            self.generate_json(404, serde_json::json!({ "error": "not found" }))
        } else {
            for page in &self.pages {
                if page.url() == path {
                    let (state, body) = match page.generate(Request::new(request)).await {
                        Ok(response) => (200, response),
                        Err(e) => (500, format!("<h1>Internal server error</h1>{:?}", e)),
                    };
                    return self.generate_html(state, body);
                }
            }
            self.generate_html(404, "Not found")
        }
    }
}

struct PageHandler<T, P> {
    _state: PhantomData<T>,
    _page: PhantomData<P>,
}

impl<T, P> Default for PageHandler<T, P> {
    fn default() -> Self {
        Self {
            _state: PhantomData,
            _page: PhantomData,
        }
    }
}

impl<T, P> Clone for PageHandler<T, P> {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl<T, P> PageHandler<T, P>
where
    T: Send + Sync + 'static,
    P: Page<T> + Sync + 'static,
{
    pub fn boxed() -> Box<dyn PageHandlerTrait<T>> {
        Box::new(Self::default())
    }
}

#[async_trait]
trait PageHandlerTrait<T>: Send + Sync {
    fn url(&self) -> &str;
    async fn generate(&self, request: Request<T>) -> Result<String>;
    async fn generate_json(&self, request: Request<T>) -> Result<serde_json::Value>;
}

#[async_trait]
impl<T, P> PageHandlerTrait<T> for PageHandler<T, P>
where
    P: Page<T> + Sync,
    T: Send + Sync,
{
    fn url(&self) -> &str {
        P::URL
    }

    async fn generate(&self, request: Request<T>) -> Result<String> {
        let p = P::construct(request).await?;
        Ok(p.html())
    }

    async fn generate_json(&self, request: Request<T>) -> Result<serde_json::Value> {
        let p = P::construct(request).await?;
        Ok(serde_json::to_value(&p).unwrap())
    }
}

pub trait Page<T = ()>: DeserializeOwned + Serialize + Constructor<T> + Send + 'static {
    const URL: &'static str;

    fn html(self) -> String;
}

pub type Result<T = ()> = std::result::Result<T, Error>;

#[async_trait]
pub trait Constructor<T = ()>: Sized {
    async fn construct(req: Request<T>) -> Result<Self>;
}

pub struct Request<T> {
    #[allow(dead_code)]
    req: tide::Request<T>,
}

impl<T> Request<T> {
    fn new(req: tide::Request<T>) -> Self {
        Self { req }
    }
}

#[async_trait]
impl<STATE, T> Constructor<STATE> for T
where
    T: Default,
    STATE: Send + Sync + 'static,
{
    async fn construct(_: Request<STATE>) -> Result<Self> {
        Ok(Self::default())
    }
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    String(String),
    Tide(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<String> for Error {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

pub trait TableRow: Clone {
    fn id(&self) -> String;
}
