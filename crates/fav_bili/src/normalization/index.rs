use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IndexOuput {
    pub video: Vec<IndexVideo>,
    pub audio: Vec<IndexAudio>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IndexVideo {
    pub id: i64,
    pub base_url: Url,
    pub backup_url: Vec<Url>,
    pub bandwidth: i64,
    pub codecid: i64,
    pub md5: String,
    pub size: u64,
    pub audio_id: i64,
    pub no_rexcode: bool,
    pub frame_rate: String,
    pub width: i64,
    pub height: i64,
    pub widevinePssh: String,
    pub bilidrmUri: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IndexAudio {
    pub id: i64,
    pub base_url: Url,
    pub backup_url: Vec<Url>,
    pub bandwidth: i64,
    pub codecid: i64,
    pub md5: String,
    pub size: u64,
    pub audio_id: i64,
    pub no_rexcode: bool,
    pub frame_rate: String,
    pub width: i64,
    pub height: i64,
    pub widevinePssh: String,
    pub bilidrmUri: String,
}
