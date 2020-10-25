use crate::events::event_reader::EventReader;
use crate::events::LogEvent;

use chrono::prelude::*;
use chrono::{DateTime, Duration, DurationRound, Utc};
use fehler::*;
use itertools::Itertools as _;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewsRequest {
    from_date: DateTime<Utc>,
    to_date: DateTime<Utc>,
    group_by_duration: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ViewsData {
    rows: Vec<ViewsDataRow>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ViewsDataRow {
    date: DateTime<Utc>,
    views: usize,
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
    pub fn get_views_data(&self, request: &ViewsRequest) -> ViewsData {
        let time_range = request.from_date.timestamp_millis()..request.to_date.timestamp_millis();
        let dur = request
            .group_by_duration
            .as_ref()
            .map(parse_duration)
            .unwrap_or_else(|| Duration::hours(1));

        let events = self.event_reader.iter::<Value>();
        let events = events
            .filter_map(|e| e.ok())
            .filter(|r| r.category == "redirect")
            .filter(|r| time_range.contains(&r.id.timestamp_millis()));

        let mut data = ViewsData {
            rows: vec![],
        };
        for (dt, group) in
            &events.group_by(|event| event.to_datetime().duration_trunc(dur).unwrap())
        {
            data.rows.push(ViewsDataRow{
                date: dt,
                views: group.count(),
            });
        }
        data
    }
}

impl LogEvent<Value> {
    fn to_datetime(&self) -> DateTime<Utc> {
        let datetime = self.id.timestamp_millis();
        // todo: can i create with ms?
        DateTime::from_utc(NaiveDateTime::from_timestamp(datetime / 1000, 0), Utc)
    }
}

fn parse_duration(_dur: &String) -> Duration {
    unimplemented!()
}
