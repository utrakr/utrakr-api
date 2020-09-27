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

    pub fn iter(&self) -> LogEventsIter {
        let walker = WalkDir::new(&self.folder)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(is_event_log);

        LogEventsIter {
            walker: Box::new(walker),
            cur: None,
        }
    }
}

pub struct LogEventsIter {
    walker: Box<dyn Iterator<Item = DirEntry>>,
    cur: Option<Box<dyn Iterator<Item = LogEvent>>>,
}

impl Iterator for LogEventsIter {
    type Item = LogEvent;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.cur.is_none() {
                self.cur = self.read_next_file();
            }

            if let Some(cur) = &mut self.cur {
                if let Some(log_event) = cur.next() {
                    // found a log event we are good
                    return Some(log_event);
                } else {
                    // did not find one, the current file is done
                    self.cur = None
                }
            } else {
                // we have run out of new files
                return None;
            }
        }
    }
}

impl LogEventsIter {
    fn read_next_file(&mut self) -> Option<Box<dyn Iterator<Item = LogEvent>>> {
        self.walker.next().map(|e| entry_to_log_entry_iterator(e))
    }
}

fn entry_to_log_entry_iterator(entry: DirEntry) -> Box<dyn Iterator<Item = LogEvent>> {
    let data = BufReader::new(File::open(entry.path()).unwrap());
    let deserializer = Deserializer::from_reader(data).into_iter::<LogEvent>();
    Box::new(deserializer.filter_map(|e| e.ok()))
}

fn is_event_log(e: &DirEntry) -> bool {
    e.file_name()
        .to_str()
        .map(|e| e.ends_with(".events.json"))
        .unwrap_or(false)
}
