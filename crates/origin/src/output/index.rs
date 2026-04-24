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
    pub fn update_video(&mut self, video: &Video) -> &mut Self {
        self.id = video.id;
        self.base_url = video.base_url.clone();
        self.backup_url = video.backup_url.clone();
        self.bandwidth = video.bandwidth;
        self.codecid = video.codecid;
        self.audio_id = 0;
        self.no_rexcode = false;
        self.frame_rate = video.frame_rate.clone();
        self.width = video.width;
        self.height = video.height;
        self.widevine_pssh = "".to_string();
        self.bilidrm_uri = "".to_string();

        self
    }

    pub fn update_audio_id(&mut self, audio_id: i64) -> &mut Self {
        self.audio_id = audio_id;
        self
    }
    
    pub fn update_md5_size(&mut self, md5: String, size: u64) -> &mut Self {
        self.md5 = md5;
        self.size = size;

        self
    }
}

impl Default for IndexVideo {
    fn default() -> Self {
        Self {
            id: Default::default(),
            base_url: Url::parse("http://127.0.0.0").expect("default url error"),
            backup_url: Default::default(),
            bandwidth: Default::default(),
            codecid: Default::default(),
            md5: Default::default(),
            size: Default::default(),
            audio_id: Default::default(),
            no_rexcode: Default::default(),
            frame_rate: Default::default(),
            width: Default::default(),
            height: Default::default(),
            widevine_pssh: Default::default(),
            bilidrm_uri: Default::default(),
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
    pub fn update_audio(&mut self, audio: &Audio) -> &mut Self {
        self.id = audio.id;
        self.base_url = audio.base_url.clone();
        self.backup_url = audio.backup_url.clone();
        self.bandwidth = audio.bandwidth;
        self.codecid = audio.codecid;
        self.audio_id = 0;
        self.no_rexcode = false;
        self.frame_rate = audio.frame_rate.clone();
        self.width = audio.width;
        self.height = audio.height;
        self.widevine_pssh = "".to_string();
        self.bilidrm_uri = "".to_string();

        self
    }

    pub fn update_md5_size(&mut self, md5: String, size: u64) -> &mut Self {
        self.md5 = md5;
        self.size = size;

        self
    }
}

impl Default for IndexAudio {
    fn default() -> Self {
        Self {
            id: Default::default(),
            base_url: Url::parse("http://127.0.0.0").expect("default url error"),
            backup_url: Default::default(),
            bandwidth: Default::default(),
            codecid: Default::default(),
            md5: Default::default(),
            size: Default::default(),
            audio_id: Default::default(),
            no_rexcode: Default::default(),
            frame_rate: Default::default(),
            width: Default::default(),
            height: Default::default(),
            widevine_pssh: Default::default(),
            bilidrm_uri: Default::default(),
        }
    }
}
