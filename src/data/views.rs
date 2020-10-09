use crate::events::event_reader::EventReader;
use crate::events::LogEvent;
use crate::RedirectEvent;
use anyhow::Result;
use chrono::prelude::*;
use chrono::{DateTime, Duration, DurationRound, Utc};
use fehler::*;
use itertools::Itertools as _;
use std::path::PathBuf;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewsRequest {
    from_date: DateTime<Utc>,
    to_date: DateTime<Utc>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ViewsData {
    x: Vec<DateTime<Utc>>,
    y: Vec<usize>,
}

#[derive(Clone)]
pub struct ViewsDao {
    event_reader: EventReader,
}

impl ViewsDao {
    pub fn new(event_reader: EventReader) -> Self {
        ViewsDao { event_reader }
    }

    pub fn from_path(path: &PathBuf) -> Self {
        ViewsDao::new(EventReader::new(path))
    }

    #[throws(anyhow::Error)]
    pub fn get_views_data(&self, _request: &ViewsRequest) -> ViewsData {
        let events = self.event_reader.iter();
        let events = events.filter(filter_cat).filter_map(|e| e.ok());

        let mut data = ViewsData {
            x: vec![],
            y: vec![],
        };
        for (dt, group) in &events.group_by(hour) {
            data.x.push(dt.clone());
            data.y.push(group.count());
        }
        data
    }
}

fn filter_cat(row: &Result<LogEvent<RedirectEvent>>) -> bool {
    row.as_ref()
        .map(|e| e.category == "redirect")
        .unwrap_or(false)
}

fn hour<T>(event: &LogEvent<T>) -> DateTime<Utc> {
    event
        .to_datetime()
        .duration_trunc(Duration::hours(1))
        .unwrap() // todo: okay to round here?
}

impl<T> LogEvent<T> {
    fn to_datetime(&self) -> DateTime<Utc> {
        let datetime = self.id.timestamp_millis();
        // todo: can i create with ms?
        DateTime::from_utc(NaiveDateTime::from_timestamp(datetime / 1000, 0), Utc)
    }
}
