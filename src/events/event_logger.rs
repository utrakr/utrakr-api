use std::path::PathBuf;

use async_std::fs::File;
use async_std::io::BufWriter;
use async_std::prelude::*;
use async_std::sync::{Arc, Mutex};
use fehler::*;
use serde::Serialize;


use crate::events::LogEvent;
use crate::ulid::{Ulid, UlidGenerator};

const VERSION: &str = "20200603";

#[derive(Clone)]
pub struct EventLogger {
    ulid_generator: Arc<Mutex<UlidGenerator>>,
    logger_id: Ulid,
    app: String,
    state: Arc<Mutex<EventLoggerOutputState>>,
}

pub struct EventLoggerOutputState {
    prev_ulid: Ulid,
    folder: PathBuf,
    file: Option<File>,
}

impl EventLogger {
    #[throws(anyhow::Error)]
    pub async fn new(
        folder: &PathBuf,
        app: &str,
        ulid_generator: Arc<Mutex<UlidGenerator>>,
    ) -> EventLogger {
        let prev_ulid = Ulid::default();
        let state = Arc::new(Mutex::new(EventLoggerOutputState {
            prev_ulid,
            folder: folder.clone(),
            file: None,
        }));
        let logger_id = ulid_generator.lock().await.generate()?;
        EventLogger {
            ulid_generator,
            logger_id,
            app: app.to_owned(),
            state,
        }
    }

    #[throws(anyhow::Error)]
    async fn write_event<T: Serialize>(&self, ulid: Ulid, event: T) -> () {
        let mut state = self.state.lock().await;
        let prev = &state.prev_ulid.to_string()[..6];
        let now = &ulid.to_string()[..6];

        if prev != now || state.file.is_none() {
            let mut file = state.folder.clone();
            file.push(format!("{}", now));
            std::fs::create_dir_all(&file)?;
            state.prev_ulid = ulid;
            file.push(format!(
                "{}.v{}.{}.events.json",
                ulid, VERSION, self.logger_id
            ));

            let f = File::create(&file).await?;
            state.file = Some(f);
        }

        let file = state.file.as_ref().unwrap();
        let mut outs = BufWriter::new(file);
        outs.write_all(&serde_json::to_vec(&event)?).await?;
        outs.write(b"\n").await?;
        outs.flush().await?;
    }

    #[throws(anyhow::Error)]
    pub async fn log_event<T>(&self, category: &str, event: &T) -> ()
    where
        T: ?Sized + Serialize,
    {
        let ulid = self.ulid_generator.lock().await.generate()?;
        // todo: way to use serde but make sure that it is in this exact order?
        let event = LogEvent {
            id: ulid,
            app: self.app.to_string(),
            category: category.to_string(),
            event,
        };
        self.write_event(ulid, event).await?;
    }
}
