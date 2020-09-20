use anyhow::Context;
use fehler::*;
use redis::AsyncCommands;

use crate::id_generator::IdGenerator;
use crate::utils::trim_trailing_slash;
use crate::AppConfig;

#[derive(Clone)]
pub struct UrlDao {
    redis_client: redis::Client,
    id_generator: IdGenerator,
    default_base_url: String,
}

pub struct UrlDaoConfig {
    redis_urls_client_conn: String,
    default_base_url: String,
}

pub trait IntoUrlDaoConfig {
    fn into_url_dao_config(self) -> UrlDaoConfig;
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct MicroUrlInfo {
    pub base_url: String,
    pub id: String,
    pub micro_url: String,
}

impl IntoUrlDaoConfig for &AppConfig {
    fn into_url_dao_config(self) -> UrlDaoConfig {
        UrlDaoConfig {
            redis_urls_client_conn: self.redis_urls_client_conn.to_owned(),
            default_base_url: format!(
                "{}://{}",
                if self.cookie_secure { "https" } else { "http" },
                self.default_base_host
            ),
        }
    }
}

impl UrlDao {
    #[throws(anyhow::Error)]
    pub fn new<T: IntoUrlDaoConfig>(config: T) -> UrlDao {
        let url_config: UrlDaoConfig = config.into_url_dao_config();
        let redis_client = redis::Client::open(url_config.redis_urls_client_conn.as_str())?;
        let id_generator = IdGenerator::new(8);
        let default_base_url = trim_trailing_slash(&url_config.default_base_url);

        UrlDao {
            redis_client,
            id_generator,
            default_base_url,
        }
    }

    #[throws(anyhow::Error)]
    pub async fn create_micro_url(&self, long_url: &str) -> MicroUrlInfo {
        info!("create micro url of [{}]", long_url);

        let id = self.id_generator.gen_id();
        debug!("created id [{}] for long url [{}]", &id, long_url);

        let mut con = self
            .redis_client
            .get_async_connection()
            .await
            .context(format!(
                "unable to get connection to redis, {:?}",
                self.redis_client
            ))?;
        con.set(&id, long_url).await?;

        let micro_url = format!("{}/{}", self.default_base_url, &id);
        MicroUrlInfo {
            base_url: self.default_base_url.to_string(),
            id,
            micro_url,
        }
    }

    #[throws(anyhow::Error)]
    pub async fn get_micro_url(&self, id: &str) -> Option<String> {
        info!("get long url from micro id [{}]", id);
        let mut con = self.redis_client.get_async_connection().await?;

        let long_url = con.get(id).await?;
        match long_url {
            Some(ref u) => debug!("found id [{}] with long url [{}]", &id, u),
            None => debug!("unable to find id [{}]", &id),
        }
        long_url
    }
}
