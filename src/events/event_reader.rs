use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use fehler::*;
use serde_json::Deserializer;
use walkdir::{DirEntry, WalkDir};

use crate::ulid::Ulid;

pub struct EventReader {
    folder: PathBuf,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct LogEvent {
    #[serde(alias = "_")]
    id: Ulid,
    #[serde(alias = "_a")]
    app: String,
    #[serde(alias = "_c")]
    category: String,
    event: String,
}

impl EventReader {
    pub fn new(folder: &PathBuf) -> EventReader {
        EventReader {
            folder: folder.clone(),
        }
    }

    #[throws(anyhow::Error)]
    pub fn events_iter(&self) -> u64 {
        let mut events = 0;
        let walker = WalkDir::new(&self.folder).into_iter();
        for entry in walker {
            let entry = entry?;
            if entry
                .path()
                .to_str()
                .map(|s| s.ends_with(".events.json"))
                .unwrap_or(false)
            {
                let data = BufReader::new(File::open(entry.path())?);
                for log_event in Deserializer::from_reader(data).into_iter::<LogEvent>() {
                    let log_event = log_event?;
                    println!("{:?}", log_event);
                    events += 1;
                }
            }
        }
        events
    }
}

fn is_event_log(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.ends_with(".events.json"))
        .unwrap_or(false)
}
