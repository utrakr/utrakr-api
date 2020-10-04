use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde_json::Deserializer;
use walkdir::{DirEntry, WalkDir};

use crate::events::LogEvent;

use anyhow::Error;
use anyhow::Result;
use either::Either;
use std::iter::once;

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

    pub fn iter<T>(&self) -> impl Iterator<Item = Result<LogEvent<T>>>
    where
        T: DeserializeOwned,
    {
        let r = self.files_iter().map(entry_to_log_entry_iterator);
        flatten_nested_results(r)
    }
}

fn entry_to_log_entry_iterator<T>(
    entry: DirEntry,
) -> Result<impl Iterator<Item = Result<LogEvent<T>>>>
where
    T: DeserializeOwned,
{
    let data = BufReader::new(File::open(entry.path())?);
    let deserializer = Deserializer::from_reader(data).into_iter::<LogEvent<T>>();
    Ok(deserializer.map(|r| r.map_err(Error::new)))
}

fn is_event_log(e: &DirEntry) -> bool {
    e.file_name()
        .to_str()
        .map(|e| e.ends_with(".events.json"))
        .unwrap_or(false)
}

fn flatten_nested_results<T, E, IterInner, IterOuter>(
    iter_outer: IterOuter,
) -> impl Iterator<Item = Result<T, E>>
where
    IterOuter: Iterator<Item = Result<IterInner, E>>,
    IterInner: Iterator<Item = Result<T, E>>,
{
    iter_outer.flat_map(|iter_inner_result| match iter_inner_result {
        Ok(iter_inner) => Either::Right(iter_inner),
        Err(err) => Either::Left(once(Err(err))),
    })
}
