#[macro_use]
extern crate log;

use crate::event_logger::EventLogger;
use crate::url_dao::{UrlDao, MicroUrlInfo};
use async_std::sync::{Arc, Mutex};
use cookie::Cookie;
use http_types::headers::HeaderValue;
use structopt::StructOpt;
use tide::security::{CorsMiddleware, Origin};
use tide::{Redirect, Request, Response, StatusCode};

mod event_logger;
mod id_generator;
mod ulid;
mod url_dao;
mod utils;

use crate::ulid::UlidGenerator;

const LOG_HEADERS: [&str; 2] = ["user-agent", "referer"];

const COOKIE_NAME: &str = "_utrakr";

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ShortenResponse {
    data: MicroUrlInfo,
    request: ShortenRequest,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ShortenRequest {
    long_url: String,
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "utrakr-api")]
struct AppConfig {
    #[structopt(env)]
    redirect_homepage: String,
    #[structopt(env)]
    default_base_host: String,
    #[structopt(env, parse(try_from_str))]
    cookie_secure: bool,
    #[structopt(env)]
    redis_urls_client_conn: String,
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

            // build or save cookie
            if let Some(c) = req.cookie(COOKIE_NAME) {
                debug!("cookie: {}", c);
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
                response.set_cookie(cookie);
            };

            // read some other headers
            for header in LOG_HEADERS.iter() {
                match req.header(*header) {
                    Some(values) => {
                        for value in values {
                            debug!("{}: {}", header, value)
                        }
                    }
                    _ => (),
                }
            }

            Ok(response)
        }
        None => Ok(Response::new(StatusCode::NotFound)),
    }
}

#[async_std::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    let app_config: AppConfig = StructOpt::from_args();
    info!("loading config {:?}", app_config);

    let ulid_generator = Arc::new(Mutex::new(UlidGenerator::new()));

    let url_dao = UrlDao::new(&app_config)?;
    let redirect = Redirect::permanent(app_config.redirect_homepage.to_owned());
    let event_logger = EventLogger::new(ulid_generator.clone());

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
