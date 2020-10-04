use crate::ulid::Ulid;

pub mod event_logger;
pub mod event_reader;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct LogEvent<T> {
    #[serde(rename = "_")]
    id: Ulid,
    #[serde(rename = "_a")]
    app: String,
    #[serde(rename = "_c")]
    category: String,
    event: T,
}

#[allow(unused_variables)]
#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    use async_std::sync::{Arc, Mutex};
    use tempfile;

    use crate::events::event_logger::EventLogger;
    use crate::events::event_reader::EventReader;
    use crate::ulid::UlidGenerator;

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    struct TestEvent {
        color: String,
    }

    #[async_std::test]
    async fn test_smoke() {
        let tmp = tempfile::tempdir().unwrap().into_path();
        let gen = Arc::new(Mutex::new(UlidGenerator::new()));

        let reader = EventReader::new(&tmp);
        let writer: EventLogger = EventLogger::new(&tmp, "test", gen).await.unwrap();
        let test_event = TestEvent {
            color: "green".to_owned(),
        };
        writer.log_event("cat", &test_event).await.unwrap();
        assert_eq!(1, reader.iter::<TestEvent>().count());
        for evt in reader.iter::<TestEvent>() {
            assert_eq!("green", evt.event.color);
        }
    }

    #[async_std::test]
    async fn test_line_order() {
        let tmp = tempfile::tempdir().unwrap().into_path();
        let gen = Arc::new(Mutex::new(UlidGenerator::new()));
        let writer: EventLogger = EventLogger::new(&tmp, "test", gen).await.unwrap();

        // write some events
        for i in 0..100 {
            writer.log_event("one", &format!("two{}", i)).await.unwrap();
        }

        // make sure that the fields are in the correct order
        let reader = EventReader::new(&tmp);
        for entry in reader.files_iter() {
            let data = BufReader::new(File::open(entry.path()).unwrap());
            for line in data.lines() {
                let line = line.unwrap();
                assert!(line.starts_with("{\"_\":"));
                assert!(line.find("\"_\":") < line.find("\"_a\":"));
                assert!(line.find("\"_a\":") < line.find("\"_c\":"));
            }
        }
    }
}
