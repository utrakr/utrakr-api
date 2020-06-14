#[macro_use]
extern crate log;

use crate::event_logger::EventLogger;
use crate::url_dao::{MicroUrlInfo, UrlDao};
use async_std::sync::{Arc, Mutex};
use cookie::Cookie;
use http_types::headers::{HeaderValue, HeaderValues};
use multimap::MultiMap;
use structopt::StructOpt;
use tide::security::{CorsMiddleware, Origin};
use tide::{Redirect, Request, Response, StatusCode};

mod event_logger;
mod id_generator;
mod ulid;
mod url_dao;
mod utils;

use crate::ulid::UlidGenerator;
use std::path::PathBuf;

const LOG_HEADERS: [&str; 2] = ["user-agent", "referer"];
const COOKIE_NAME: &str = "_utrakr";
const APP_NAME: &str = "utrakr-api";

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ShortenResponse {
    data: MicroUrlInfo,
    request: ShortenRequest,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ShortenRequest {
    long_url: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct RedirectEvent {
    cookie: Option<RedirectCookieInfo>,
    headers: MultiMap<String, String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct RedirectCookieInfo {
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    domain: Option<String>,
}

impl RedirectEvent {
    fn empty() -> RedirectEvent {
        RedirectEvent {
            cookie: None,
            headers: MultiMap::new(),
        }
    }
    fn set_from_cookie(&mut self, c: &Cookie) {
        self.cookie = Some(RedirectCookieInfo {
            value: c.value().to_string(),
            // only logs on creation
            expires: c.expires().map(|t| t.to_utc().rfc3339().to_string()),
            domain: c.domain().map(|s| s.to_owned()),
        })
    }
    fn add_header_values(&mut self, header: &str, values: &HeaderValues) {
        for header_value in values {
            let value: String = header_value.to_string();
            debug!("{}: {}", header, value);
            self.headers.insert(header.to_owned(), value);
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct App {
    name: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct Startup {
    app: App,
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = APP_NAME)]
struct AppConfig {
    #[structopt(env)]
    redirect_homepage: String,
    #[structopt(env)]
    default_base_host: String,
    #[structopt(env, parse(try_from_str))]
    cookie_secure: bool,
    #[structopt(env)]
    redis_urls_client_conn: String,
    #[structopt(env, parse(try_from_str), default_value = "/tmp/utrakr-api")]
    event_log_folder: PathBuf,
}

struct AppState {
    app_config: AppConfig,
    url_dao: url_dao::UrlDao,
    event_logger: EventLogger,
    ulid_generator: Arc<Mutex<UlidGenerator>>,
}

async fn create_micro_url(mut req: Request<AppState>) -> tide::Result<Response> {
    if let Ok(request) = req.body_json::<ShortenRequest>().await {
        let url_dao = &req.state().url_dao;
        let data = url_dao
            .create_micro_url(&request.long_url)
            .await
            .map_err(|e| tide::Error::from_str(StatusCode::InternalServerError, e))?;

        let response = ShortenResponse { data, request };
        let event_logger = &req.state().event_logger;
        event_logger
            .log_event("create", &response)
            .await
            .map_err(|e| tide::Error::from_str(StatusCode::InternalServerError, e))?;

        Ok(Response::new(StatusCode::Ok).body_json(&response)?)
    } else {
        Ok(Response::new(StatusCode::UnprocessableEntity))
    }
}

async fn redirect_micro_url(req: Request<AppState>) -> tide::Result<Response> {
    let id: String = req.param("id").unwrap_or_else(|_| "".into());
    let url_dao = &req.state().url_dao;
    let domain: String = req.state().app_config.default_base_host.to_owned();
    let cookie_secure = req.state().app_config.cookie_secure;

    let found: Option<String> = url_dao
        .get_micro_url(&id)
        .await
        .map_err(|e| tide::Error::from_str(StatusCode::InternalServerError, e))?;
    match found {
        Some(long_url) => {
            let mut response = Response::redirect(long_url);
            let mut event = RedirectEvent::empty();

            // build or save cookie
            if let Some(c) = req.cookie(COOKIE_NAME) {
                debug!("cookie: {}", c);
                event.set_from_cookie(&c);
            } else {
                let mut gen = req.state().ulid_generator.lock().await;
                let mut cookie = Cookie::new(COOKIE_NAME, gen.generate()?.to_string());
                let mut now = time::now();
                now.tm_year += 1;
                cookie.set_expires(now);
                cookie.set_http_only(true);
                cookie.set_secure(cookie_secure);
                if !domain.starts_with("localhost") {
                    cookie.set_domain(domain);
                }
                event.set_from_cookie(&cookie);
                response.set_cookie(cookie);
            };

            // read headers
            for header in LOG_HEADERS.iter() {
                match req.header(*header) {
                    Some(values) => {
                        event.add_header_values(*header, values);
                    }
                    _ => (),
                }
            }

            let event_logger = &req.state().event_logger;
            event_logger
                .log_event("redirect", &event)
                .await
                .map_err(|e| tide::Error::from_str(StatusCode::InternalServerError, e))?;

            Ok(response)
        }
        None => Ok(Response::new(StatusCode::NotFound)),
    }
}

#[async_std::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    let app = App {
        name: APP_NAME.to_owned(),
    };
    let app_config: AppConfig = StructOpt::from_args();
    info!("loading config {:?}", app_config);

    let ulid_generator = Arc::new(Mutex::new(UlidGenerator::new()));

    let url_dao = UrlDao::new(&app_config)?;
    let redirect = Redirect::permanent(app_config.redirect_homepage.to_owned());
    let event_logger: EventLogger =
        EventLogger::new(app_config.event_log_folder.clone(), ulid_generator.clone()).await?;

    event_logger.log_event("startup", &Startup { app }).await?;
    let app_state = AppState {
        url_dao,
        app_config,
        event_logger,
        ulid_generator,
    };

    // app
    let mut app = tide::with_state(app_state);
    app.at("/").get(redirect).post(create_micro_url);
    app.at("/:id").get(redirect_micro_url);

    // cors
    let cors = CorsMiddleware::new()
        .allow_methods("GET, POST, OPTIONS".parse::<HeaderValue>().unwrap())
        .allow_origin(Origin::from("*"))
        .allow_credentials(false);
    app.middleware(cors);

    // listen
    app.listen("0.0.0.0:8080").await?;
    Ok(())
}
