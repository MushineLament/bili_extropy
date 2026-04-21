use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Deserialize)]
pub struct DashResp {
    pub data: DashData,
}

#[derive(Debug, Deserialize)]
pub struct DashData {
    pub dash: Dash,
    pub timelength: i64,
}

#[derive(Debug, Deserialize)]
pub struct Dash {
    pub video: VecDeque<Video>,
    pub audio: VecDeque<Audio>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Video {
    pub id: i64,
    pub base_url: Url,
    pub backup_url: Vec<Url>,
    pub bandwidth: i64,
    pub codecid: i64,
    pub frame_rate: String,
    pub width: i64,
    pub height: i64,
}

#[derive(Debug, Deserialize)]
pub struct Audio {
    pub id: i64,
    pub base_url: Url,
    pub backup_url: Vec<Url>,
    pub bandwidth: i64,
    pub codecid: i64,
    pub frame_rate: String,
    pub width: i64,
    pub height: i64,
}
