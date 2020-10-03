use crate::ulid::Ulid;
use serde_json::Value;

pub mod event_logger;
pub mod event_reader;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct LogEvent {
    #[serde(rename = "_")]
    id: Ulid,
    #[serde(rename = "_a")]
    app: String,
    #[serde(rename = "_c")]
    category: String,
    event: Value,
}

#[allow(unused_variables)]
#[cfg(test)]
mod tests {
    use async_std::sync::{Arc, Mutex};
    use fehler::*;
    use tempfile;

    use crate::events::event_logger::EventLogger;
    use crate::events::event_reader::EventReader;
    use crate::ulid::UlidGenerator;
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use serde_json::from_value;

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    struct TestEvent {
        color: String
    }

    #[throws(anyhow::Error)]
    #[async_std::test]
    async fn test_smoke() {
        let tmp = tempfile::tempdir()?.into_path();
        let gen = Arc::new(Mutex::new(UlidGenerator::new()));

        let reader = EventReader::new(&tmp);
        let writer: EventLogger = EventLogger::new(&tmp, "test", gen).await?;
        writer.log_event("cat", &TestEvent{color: "green".to_owned()}).await?;
        assert_eq!(1, reader.iter().count());
        for evt in reader.iter() {
            let te: TestEvent = from_value(evt.event)?;
            assert_eq!("green", te.color);
        }
    }

    #[throws(anyhow::Error)]
    #[async_std::test]
    async fn test_line_order() {
        let tmp = tempfile::tempdir()?.into_path();
        let gen = Arc::new(Mutex::new(UlidGenerator::new()));
        let writer: EventLogger = EventLogger::new(&tmp, "test", gen).await?;

        // write some events
        for i in 0..100 {
            writer.log_event("one", &format!("two{}", i)).await?;
        }

        // make sure that the fields are in the correct order
        let reader = EventReader::new(&tmp);
        for entry in reader.files_iter() {
            let data = BufReader::new(File::open(entry.path())?);
            for line in data.lines() {
                let line = line?;
                assert!(line.find("\"_\":") < line.find("\"_a\":"));
                assert!(line.find("\"_a\":") < line.find("\"_c\":"));
            }
        }
    }
}
