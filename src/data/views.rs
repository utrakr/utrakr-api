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
    #[serde(with = "parse_duration")]
    group_by_duration: Option<Duration>,
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
    pub fn get_views_data(&self, request: &ViewsRequest) -> ViewsData {
        let time_range = request.from_date.timestamp_millis()..request.to_date.timestamp_millis();
        let dur = request.group_by_duration.unwrap_or_else(|| Duration::hours(1));

        let events = self.event_reader.iter::<Value>();
        let events = events
            .filter_map(|e| e.ok())
            .filter(|r| r.category == "redirect")
            .filter(|r| time_range.contains(&r.id.timestamp_millis()));

        let mut data = ViewsData {
            x: vec![],
            y: vec![],
        };
        for (dt, group) in
            &events.group_by(|event| event.to_datetime().duration_trunc(dur).unwrap())
        {
            data.x.push(dt);
            data.y.push(group.count());
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

mod parse_duration {
    use chrono::{Duration};
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(duration) = duration {
            serializer.serialize_str(&duration.to_string())
        } else {
            serializer.serialize_none()
        }
    }

    pub fn deserialize<'de, D>(_deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        unimplemented!()
    }
}
