pub mod event_logger;
pub mod event_reader;

#[allow(unused_variables)]
#[cfg(test)]
mod tests {
    use async_std::sync::{Arc, Mutex};
    use fehler::*;
    use tempfile;

    use crate::events::event_logger::EventLogger;
    use crate::events::event_reader::EventReader;
    use crate::ulid::UlidGenerator;

    #[throws(anyhow::Error)]
    #[async_std::test]
    async fn test_smoke() {
        let tmp = tempfile::tempdir()?.into_path();
        let gen = Arc::new(Mutex::new(UlidGenerator::new()));

        let reader = EventReader::new(&tmp);
        let writer: EventLogger = EventLogger::new(&tmp, "test", gen).await?;
        writer.log_event("cat", "event").await?;
        assert_eq!(1, reader.iter().count());
        for evt in reader.iter() {
            println!("{:?}", evt);
        }
    }
}
