use crate::ulid::{Ulid, UlidGenerator};
use async_std::fs::File;
use async_std::io::BufWriter;
use async_std::prelude::*;
use async_std::sync::{Arc, Mutex};
use fehler::*;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::PathBuf;

pub struct EventLogger {
    ulid_generator: Arc<Mutex<UlidGenerator>>,
    logger_id: Ulid,
    state: Mutex<EventLoggerOutputState>,
}

pub struct EventLoggerOutputState {
    prev_ulid: Ulid,
    folder: PathBuf,
    file: Option<File>,
}

impl EventLogger {
    #[throws(anyhow::Error)]
    pub async fn new(folder: PathBuf, ulid_generator: Arc<Mutex<UlidGenerator>>) -> EventLogger {
        let prev_ulid = Ulid::default();
        let state = Mutex::new(EventLoggerOutputState {
            prev_ulid,
            folder,
            file: None,
        });
        let logger_id = ulid_generator.lock().await.generate()?;
        EventLogger {
            ulid_generator,
            logger_id,
            state,
        }
    }

    #[throws(anyhow::Error)]
    async fn write_event(&self, ulid: Ulid, event: Value) -> () {
        let mut state = self.state.lock().await;
        let prev = &state.prev_ulid.to_string()[..6];
        let now = &ulid.to_string()[..6];

        if prev != now || state.file.is_none() {
            let mut file = state.folder.clone();
            file.push(format!("{}", now));
            std::fs::create_dir_all(&file)?;
            state.prev_ulid = ulid;
            file.push(format!("{}.{}.events.json", ulid, self.logger_id));

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
    pub async fn log_event<T>(&self, ns: &str, event: &T) -> ()
    where
        T: ?Sized + Serialize,
    {
        let ulid = self.ulid_generator.lock().await.generate()?;
        let event = json!({
            "_": ulid,
            "_ns": ns,
            "event": event,
        });

        self.write_event(ulid, event).await?;
    }
}
