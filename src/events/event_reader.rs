use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde_json::Deserializer;
use walkdir::{DirEntry, WalkDir};

use crate::events::LogEvent;

pub struct EventReader {
    folder: PathBuf,
}

impl EventReader {
    pub fn new(folder: &PathBuf) -> EventReader {
        EventReader {
            folder: folder.clone(),
        }
    }

    pub fn files_iter(&self) -> impl Iterator<Item = DirEntry> {
        WalkDir::new(&self.folder)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(is_event_log)
    }

    pub fn iter<T>(&self) -> impl Iterator<Item = LogEvent<T>>
    where
        T: DeserializeOwned,
    {
        self.files_iter()
            .map(|e| entry_to_log_entry_iterator(e))
            .filter_map(|e| e.ok())
            .flatten()
    }
}

fn entry_to_log_entry_iterator<T>(
    entry: DirEntry,
) -> anyhow::Result<impl Iterator<Item = LogEvent<T>>>
where
    T: DeserializeOwned,
{
    let data = BufReader::new(File::open(entry.path())?);
    let deserializer = Deserializer::from_reader(data).into_iter::<LogEvent<T>>();
    Ok(deserializer.filter_map(|e| e.ok()))
}

fn is_event_log(e: &DirEntry) -> bool {
    e.file_name()
        .to_str()
        .map(|e| e.ends_with(".events.json"))
        .unwrap_or(false)
}
