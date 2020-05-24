#[macro_use]
extern crate log;

use crate::url_dao::UrlDao;
use cookie::Cookie;
use structopt::StructOpt;
use tide::{Redirect, Request, Response, StatusCode};
use time;
use ulid::Ulid;

mod id_generator;
mod url_dao;
mod utils;

const LOG_HEADERS: [&'static str; 2] = ["user-agent", "referer"];

const COOKIE_NAME: &'static str = "_utrakr";

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ShortenResponse {
    micro_url: String,
    request: ShortenRequest,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ShortenRequest {
    long_url: String,
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "utraker-api")]
struct AppConfig {
    #[structopt(env)]
    homepage: String,
    #[structopt(env)]
    default_base_host: String,
    #[structopt(env, parse(try_from_str))]
    default_secure_host: bool,
    #[structopt(env)]
    redis_urls_client_conn: String,
}

struct AppState {
    app_config: AppConfig,
    url_dao: url_dao::UrlDao,
}

async fn create_micro_url(mut req: Request<AppState>) -> tide::Result<Response> {
    if let Ok(request) = req.body_form::<ShortenRequest>().await {
        let url_dao = &req.state().url_dao;
        let micro_url = url_dao
            .create_micro_url(&request.long_url)
            .await
            .map_err(|e| tide::Error::from_str(StatusCode::InternalServerError, e))?;
        let response = ShortenResponse { micro_url, request };
        Ok(Response::new(StatusCode::Ok).body_json(&response)?)
    } else {
        Ok(Response::new(StatusCode::UnprocessableEntity))
    }
}

async fn redirect_micro_url(req: Request<AppState>) -> tide::Result<Response> {
    let id: String = req.param("id").unwrap_or("".into());
    let url_dao = &req.state().url_dao;
    let domain: String = req.state().app_config.default_base_host.to_owned();
    let cookie_secure = req.state().app_config.default_secure_host;

    let found: Option<String> = url_dao
        .get_micro_url(&id)
        .await
        .map_err(|e| tide::Error::from_str(StatusCode::InternalServerError, e))?;
    match found {
        Some(long_url) => {
            let mut response = Response::redirect(long_url);

            // build or save cookie
            if let Some(c) = req.cookie(COOKIE_NAME) {
                debug!("cookie: {:#?}", c);
            } else {
                let mut cookie = Cookie::new(COOKIE_NAME, Ulid::new().to_string());
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
                debug!("{}: {:#?}", header, req.header(*header));
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
    info!("loading app config {:?}", app_config);

    let app_state = AppState {
        url_dao: UrlDao::new(app_config.clone().into())?,
        app_config: app_config.clone(),
    };

    let mut app = tide::with_state(app_state);
    app.at("/")
        .get(Redirect::permanent(app_config.homepage))
        .post(create_micro_url);
    app.at("/:id").get(redirect_micro_url);
    app.listen("127.0.0.1:8080").await?;
    Ok(())
}
