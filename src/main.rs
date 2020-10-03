#[macro_use]
extern crate log;

use std::path::PathBuf;

use async_std::sync::{Arc, Mutex};
use fehler::*;
use http_types;
use http_types::headers::{HeaderValue, HeaderValues};
use multimap::MultiMap;
use structopt::StructOpt;
use tide::{Body, Redirect, Request, Response, StatusCode};
use tide::http::Cookie;
use tide::log::LevelFilter;
use tide::security::{CorsMiddleware, Origin};
use time::{Duration, OffsetDateTime};

use crate::dao::url_dao;
use crate::dao::url_dao::{MicroUrlInfo, UrlDao};
use crate::data::views::{get_views_data, ViewsData, ViewsRequest};
use crate::events::event_logger::EventLogger;
use crate::google_auth::{get_claim_from_google, GoogleClaims};
use crate::ulid::UlidGenerator;

mod dao;
mod data;
mod events;
mod google_auth;
mod id_generator;
mod ulid;
mod utils;

const LOG_HEADERS: [&str; 2] = ["user-agent", "referer"];
const COOKIE_NAME: &str = "_utrakr";
const APP_NAME: &str = "utrakr-api";

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ShortenResponse {
    data: MicroUrlInfo,
    request: ShortenRequest,
    google_auth: Option<GoogleClaims>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ShortenRequest {
    long_url: String,
    id_token: Option<String>,
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
            expires: c.expires().map(|t| t.to_string()),
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
    #[structopt(env, default_value = "debug")]
    log_level: LevelFilter,
    #[structopt(env, default_value = "http://localhost:1111")]
    redirect_homepage: String,
    #[structopt(env, default_value = "localhost:8080")]
    default_base_host: String,
    #[structopt(env, parse(try_from_str), default_value = "false")]
    cookie_secure: bool,
    #[structopt(env, default_value = "redis://127.0.0.1/")]
    redis_urls_client_conn: String,
    #[structopt(env, parse(try_from_str), default_value = "/tmp/utrakr-api")]
    event_log_folder: PathBuf,
}

#[derive(Clone)]
struct AppState {
    app_config: AppConfig,
    url_dao: url_dao::UrlDao,
    event_logger: EventLogger,
    ulid_generator: Arc<Mutex<UlidGenerator>>,
}

async fn create_micro_url(mut req: Request<AppState>) -> tide::Result<Response> {
    if let Ok(request) = req.body_json::<ShortenRequest>().await {
        let url_dao = &req.state().url_dao;
        let google_auth = if let Some(ref tk) = request.id_token {
            get_claim_from_google(tk)
        } else {
            None
        };

        let data = url_dao
            .create_micro_url(&request.long_url)
            .await
            .map_err(|e| tide::Error::from_str(StatusCode::InternalServerError, e))?;

        let response = ShortenResponse {
            data,
            request,
            google_auth,
        };
        let event_logger = &req.state().event_logger;
        event_logger
            .log_event("create", &response)
            .await
            .map_err(|e| tide::Error::from_str(StatusCode::InternalServerError, e))?;

        Ok(Response::builder(StatusCode::Ok)
            .body(Body::from_json(&response)?)
            .build())
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
        .map_err(|e| tide::Error::new(StatusCode::InternalServerError, e))?;
    match found {
        Some(long_url) => {
            let mut response: Response = Redirect::temporary(long_url).into();
            let mut event = RedirectEvent::empty();

            // build or save cookie
            if let Some(c) = req.cookie(COOKIE_NAME) {
                debug!("cookie: {}", c);
                event.set_from_cookie(&c);
            } else {
                let mut gen = req.state().ulid_generator.lock().await;
                let mut now = OffsetDateTime::now_utc();
                now += Duration::weeks(52);
                let cookie_builder = Cookie::build(COOKIE_NAME, gen.generate()?.to_string())
                    .expires(now)
                    .http_only(true)
                    .secure(cookie_secure);

                let cookie = (if !domain.starts_with("localhost") {
                    cookie_builder.domain(domain)
                } else {
                    cookie_builder
                })
                .finish();
                event.set_from_cookie(&cookie);
                response.insert_cookie(cookie);
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

async fn ruok(_req: Request<AppState>) -> tide::Result<Response> {
    Ok(Response::builder(StatusCode::Ok)
        .body("imok".to_owned())
        .build())
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ViewsResponse {
    request: ViewsRequest,
    account: UserAccount,
    data: ViewsData,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct UserAccount {
    email: String,
}

#[throws(http_types::Error)]
async fn views(req: Request<AppState>) -> Response {
    let request: ViewsRequest = req.query()?;
    if let Some(account) = read_auth(req) {
        let data = get_views_data(&request)?;
        Response::builder(StatusCode::Ok)
            .body(Body::from_json(&ViewsResponse {
                account,
                request,
                data,
            })?)
            .build()
    } else {
        Response::new(StatusCode::Unauthorized)
    }
}

fn read_auth(req: Request<AppState>) -> Option<UserAccount> {
    if let Some(auth) = req.header("authorization") {
        if auth.as_str().starts_with("Bearer ") {
            let jwt = &auth.as_str()["Bearer ".len()..];
            if !jwt.is_empty() {
                let claim = get_claim_from_google(jwt);
                return claim.map(|c| UserAccount { email: c.email });
            } else {
                warn!("found bearer with no token")
            }
        } else {
            warn!("found authorization with no bearer")
        }
    }
    return None;
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    let app = App {
        name: APP_NAME.to_owned(),
    };
    let app_config: AppConfig = StructOpt::from_args();
    tide::log::with_level(app_config.log_level);

    info!("loading config {:?}", app_config);

    let ulid_generator = Arc::new(Mutex::new(UlidGenerator::new()));

    let url_dao = UrlDao::new(&app_config)?;
    let redirect = Redirect::permanent(app_config.redirect_homepage.to_owned());
    let event_logger: EventLogger = EventLogger::new(
        &app_config.event_log_folder,
        APP_NAME,
        ulid_generator.clone(),
    )
    .await?;

    event_logger.log_event("startup", &Startup { app }).await?;
    let app_state = AppState {
        url_dao,
        app_config,
        event_logger,
        ulid_generator,
    };

    // app
    let mut app = tide::with_state(app_state);
    app.at("/private/ruok").get(ruok);
    app.at("/").get(redirect).post(create_micro_url);
    app.at("/:id").get(redirect_micro_url);
    app.at("/api/views").get(views);

    // cors
    let cors = CorsMiddleware::new()
        .allow_methods("GET, POST, OPTIONS".parse::<HeaderValue>().unwrap())
        .allow_origin(Origin::from("*"))
        .allow_credentials(false);
    app.with(cors);

    // logs
    app.with(tide::log::LogMiddleware::new());

    // listen
    app.listen("0.0.0.0:8080").await?;
    Ok(())
}
