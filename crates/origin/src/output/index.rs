use serde::{Deserialize, Serialize};
use url::Url;

use crate::response::{Audio, Video};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
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
    pub widevine_pssh: String,
    pub bilidrm_uri: String,
}

impl IndexVideo {
    pub fn from_video(video: Video, md5: String, size: u64, audio_id: i64) -> Self {
        Self {
            id: video.id,
            base_url: video.base_url,
            backup_url: video.backup_url,
            bandwidth: video.bandwidth,
            codecid: video.codecid,
            md5,
            size,
            audio_id,
            no_rexcode: false,
            frame_rate: video.frame_rate,
            width: video.width,
            height: video.height,
            widevine_pssh: "".to_string(),
            bilidrm_uri: "".to_string(),
        }
    }
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
    pub widevine_pssh: String,
    pub bilidrm_uri: String,
}

impl IndexAudio {
    pub fn from_audio(audio: Audio, md5: String, size: u64) -> Self {
        Self {
            id: audio.id,
            base_url: audio.base_url,
            backup_url: audio.backup_url,
            bandwidth: audio.bandwidth,
            codecid: audio.codecid,
            md5,
            size,
            audio_id: 0,
            no_rexcode: false,
            frame_rate: audio.frame_rate,
            width: audio.width,
            height: audio.height,
            widevine_pssh: "".to_string(),
            bilidrm_uri: "".to_string(),
        }
    }
}
