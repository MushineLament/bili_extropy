use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Deserialize)]
pub struct DashResp {
    pub data: DashData,
}

#[derive(Debug, Deserialize)]
pub struct DashData {
    pub dash: Dash,
}

#[derive(Debug, Deserialize)]
pub struct Dash {
    pub video: Vec<Video>,
    pub audio: Vec<Audio>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Video {
    pub id: i64,
    pub base_url: Url,
    pub backup_url: Vec<Url>,
    pub bandwidth: i64,
    pub codecid: i64,
    // md5的计算是由本地进行的，因此无法从api中获取
    // pub md5: String,
    // size 同理
    // pub size: i64,
    // 关联的video id，该字段存在于video中
    // pub audio_id:i64,

    // 是否不需要重新编码(存疑，来自ai)
    // pub no_rexcode: bool,
    pub frame_rate: String,
    pub width: i64,
    pub height: i64,
    // 是 Google Widevine DRM 系统所需的初始化数据（Protection System Specific Header）。
    // 当视频需要 DRM 保护时，该字段会包含一个 Base64 编码的字符串，用于向 Widevine CDM（内容解密模块）提供解密所需的关键信息。
    // 如果该字段为空字符串（如示例中的 ""），则表示当前视频没有使用 Widevine DRM 保护，可以直接播放。
    // (来自ai)
    // pub widevinePssh: String,
    // 是 Bilibili 自定义的 DRM 相关字段，通常指向一个 URL 或资源标识，用于获取 DRM 许可证或进行其他 DRM 相关的操作。
    // 如果该字段为空，也表示该视频不需要经过额外的 DRM 流程。
    // (来自ai)
    // pub bilidrmUri: String,
}

#[derive(Debug, Deserialize)]
pub struct Audio {
    pub id: i64,
    pub base_url: Url,
    pub backup_url: Vec<Url>,
    pub bandwidth: i64,
    pub codecid: i64,
    // md5的计算是由本地进行的，因此无法从api中获取
    // pub md5: String,
    // size 同理
    // pub size: i64,
    // 关联的audio id，但一般都是它自身
    // pub audio_id: i64,
    // 是否不需要重新编码(存疑，来自ai)
    // pub no_rexcode: bool,
    pub frame_rate: String,
    pub width: i64,
    pub height: i64,
    // 是 Google Widevine DRM 系统所需的初始化数据（Protection System Specific Header）。
    // 当视频需要 DRM 保护时，该字段会包含一个 Base64 编码的字符串，用于向 Widevine CDM（内容解密模块）提供解密所需的关键信息。
    // 如果该字段为空字符串（如示例中的 ""），则表示当前视频没有使用 Widevine DRM 保护，可以直接播放。
    // (来自ai)
    // pub widevinePssh: String,
    // 是 Bilibili 自定义的 DRM 相关字段，通常指向一个 URL 或资源标识，用于获取 DRM 许可证或进行其他 DRM 相关的操作。
    // 如果该字段为空，也表示该视频不需要经过额外的 DRM 流程。
    // (来自ai)
    // pub bilidrmUri: String,
}
