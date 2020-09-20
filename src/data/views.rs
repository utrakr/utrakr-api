use chrono::{DateTime, Utc};
use fehler::*;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewsRequest {
    from_date: DateTime<Utc>,
    to_date: DateTime<Utc>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ViewsData {
    x: Vec<DateTime<Utc>>,
    y: Vec<u64>,
}

#[throws(anyhow::Error)]
pub fn get_views_data(_request: &ViewsRequest) -> ViewsData {
    ViewsData {
        x: vec![_request.from_date, _request.to_date],
        y: vec![3, 5],
    }
}
