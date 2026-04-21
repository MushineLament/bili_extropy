use std::str::FromStr;

use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EntryOuput {
    /// 媒体类型，2 通常表示普通视频（也可能是其他数值区分不同类型）。
    pub media_type: i64,
    /// 是否包含 DASH 音频流（true 表示有独立音频流）。
    pub has_dash_audio: bool,
    /// 缓存是否完成（true 表示已完整下载）。
    pub is_completed: bool,
    /// 视频总大小（字节）。
    pub total_bytes: u64,
    /// 已下载字节数。
    pub downloaded_bytes: u64,
    /// 视频标题。
    pub title: String,
    /// 视频清晰度标识（如 80 代表 1080P，数字对应 quality 值）。
    pub type_tag: String,
    /// 视频封面图片 URL。
    pub cover: Url,
    /// 当前缓存视频的清晰度值（与 type_tag 对应）。
    pub video_quality: i64,
    /// 用户偏好的清晰度（可能用于后续下载）。
    pub prefered_video_quality: i64,
    /// 猜测的总大小（通常为 0，可能用于网络波动估算）。
    pub guessed_total_bytes: i64,
    /// 视频总时长（毫秒）。
    pub total_time_milli: i64,
    /// 弹幕数量（已加载或估计值）。
    pub danmaku_count: i64,
    /// 上次更新时间戳（毫秒）。
    pub time_update_stamp: u128,
    /// 缓存创建时间戳（毫秒）。
    pub time_create_stamp: u128,
    /// 是否支持边下边播。
    pub can_play_in_advance: bool,
    /// 是否中断转换临时文件（可能用于下载后处理）。
    pub interrupt_transform_temp_file: bool,
    /// 清晰度文本描述（如 “1080P”）。
    pub quality_pithy_description: String,
    /// 清晰度上标（通常为空）。
    pub quality_superscript: String,
    /// 是否可变分辨率（可能指自适应）。
    pub variable_resolution_ratio: bool,
    /// 缓存版本号（对应客户端版本）。
    pub cache_version_code: i64,
    /// 用户偏好的音频质量（数值）。
    pub preferred_audio_quality: i64,
    /// 当前缓存的音频质量。
    pub audio_quality: i64,
    /// 视频的 avid（AV 号，视频唯一标识）。
    pub avid: i64,
    /// 番剧 ID（非番剧为 0）。
    pub spid: i64,
    /// 剧集 ID（非番剧为 0）。
    pub season_id: i64,
    /// 视频的 BV 号（此处为空，可能是因为旧版本或仅存储 avid）。
    pub bvid: String,
    /// UP 主的用户 ID。
    pub owner_id: i64,
    /// UP 主昵称。
    pub owner_name: String,
    /// 是否为付费视频。
    pub is_charge_video: bool,
    /// 校验码（通常为 0）。
    pub verification_code: i64,
    /// 分页信息（子对象）。
    pub page_data: EntryPageData,
    /// 剧集信息（非番剧为/ null）。
    pub ep: Option<String>,
}

impl Default for EntryOuput {
    fn default() -> Self {
        Self {
            cover: Url::from_str("http://127.0.0.0/").expect("not a 127.0.0.0"),
            media_type: Default::default(),
            has_dash_audio: Default::default(),
            is_completed: Default::default(),
            total_bytes: Default::default(),
            downloaded_bytes: Default::default(),
            title: Default::default(),
            type_tag: Default::default(),
            video_quality: Default::default(),
            prefered_video_quality: Default::default(),
            guessed_total_bytes: Default::default(),
            total_time_milli: Default::default(),
            danmaku_count: Default::default(),
            time_update_stamp: Default::default(),
            time_create_stamp: Default::default(),
            can_play_in_advance: Default::default(),
            interrupt_transform_temp_file: Default::default(),
            quality_pithy_description: Default::default(),
            quality_superscript: Default::default(),
            variable_resolution_ratio: Default::default(),
            cache_version_code: Default::default(),
            preferred_audio_quality: Default::default(),
            audio_quality: Default::default(),
            avid: Default::default(),
            spid: Default::default(),
            season_id: Default::default(),
            bvid: Default::default(),
            owner_id: Default::default(),
            owner_name: Default::default(),
            is_charge_video: Default::default(),
            verification_code: Default::default(),
            page_data: Default::default(),
            ep: Default::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EntryPageData {
    /// 当前分页的 cid（视频流标识）。
    pub cid: i64,
    /// 分页码（从 1 开始）。
    pub page: i64,
    /// 上传来源（如 vupload 表示用户投稿）。
    pub from: String,
    /// 分页标题（若无分页则为视频标题）。
    pub part: String,
    /// 客户端链接（用于跳转）。
    pub link: String,
    /// 富媒体 ID（通常为空）。
    pub rich_vid: String,
    /// 是否有别名。
    pub has_alias: bool,
    /// 分区 ID（0 表示未知）。
    pub tid: i64,
    /// 视频宽度（像素）。
    pub width: i64,
    /// 视频高度（像素）。
    pub height: i64,
    /// 旋转角度（0 为正常）。
    pub rotate: i64,
    /// 下载标题（可能用于保存文件名）。
    pub download_title: String,
    /// 下载副标题。
    pub download_subtitle: String,
}
