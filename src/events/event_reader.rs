use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

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

    pub fn iter(&self) -> impl Iterator<Item = LogEvent> {
        WalkDir::new(&self.folder)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(is_event_log)
            .map(|e| entry_to_log_entry_iterator(e))
            .filter_map(|e| e.ok())
            .flatten()
    }
}

fn entry_to_log_entry_iterator(
    entry: DirEntry,
) -> anyhow::Result<impl Iterator<Item = LogEvent>> {
    let data = BufReader::new(File::open(entry.path())?);
    let deserializer = Deserializer::from_reader(data).into_iter::<LogEvent>();
    Ok(deserializer.filter_map(|e| e.ok()))
}

fn is_event_log(e: &DirEntry) -> bool {
    e.file_name()
        .to_str()
        .map(|e| e.ends_with(".events.json"))
        .unwrap_or(false)
}
