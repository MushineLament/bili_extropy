use std::{
    borrow::Cow,
    fmt::Debug,
    fs::File,
    hash::Hash,
    io::{self, Seek, SeekFrom, Write},
    panic::Location,
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicU64},
    time::Duration,
};

use api_req::ApiCaller;
use bytes::BytesMut;
use futures::future::BoxFuture;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::{IntoUrl, Response};
use tempfile::NamedTempFile;
use tokio::time::error::Elapsed;
use url::Url;

use crate::{
    api::BiliApi,
    components::fetch::FetchMedia,
    cookies::{add_cookie_jar, parse_cookies},
    output::{EntryOuput, IndexAudio, IndexVideo},
    response::{Audio, Video},
};

#[derive(Debug)]
pub struct TempFilePending {
    pub name: Cow<'static, str>,
    pub tmp: NamedTempFile,
    pub target: Vec<PathBuf>,
}

impl TempFilePending {
    pub fn new<T: Into<Cow<'static, str>>>(
        name: T,
        tmp: NamedTempFile,
        target: impl IntoIterator<Item = PathBuf>,
    ) -> Self {
        Self {
            name: name.into(),
            tmp,
            target: target.into_iter().collect(),
        }
    }

    pub fn get_size_md5(&self) -> Result<(u64, String), io::Error> {
        let file = self.tmp.as_file();

        let size = file.metadata()?.len();

        // 2. 打开一个独立的文件句柄（克隆），避免影响原句柄
        let mut file = self.tmp.as_file().try_clone()?;

        file.seek(SeekFrom::Start(0))?;

        let mut v_context = md5::Context::new();
        let mut v_buffer = [0; 8192];

        loop {
            let n = io::Read::read(&mut file, &mut v_buffer)?;
            if n == 0 {
                break;
            }
            v_context.consume(&v_buffer[..n]);
        }

        let md5_video = v_context.finalize();
        let hex_video = hex::encode(md5_video.as_ref());

        Ok((size, hex_video))
    }

    pub fn to_reopon(self) -> Result<TempFileReopen, io::Error> {
        let reopon = self.tmp.reopen()?;
        Ok(TempFileReopen {
            name: self.name,
            file: reopon,
        })
    }

    pub fn as_reopon(&self) -> Result<TempFileReopen, io::Error> {
        let reopon = self.tmp.reopen()?;
        Ok(TempFileReopen {
            name: self.name.clone(),
            file: reopon,
        })
    }
}

#[derive(Debug)]
pub struct TempFileReopen {
    pub name: Cow<'static, str>,
    pub file: File,
}

impl TempFileReopen {
    pub fn new<T: Into<Cow<'static, str>>>(name: T, tmp: File) -> Self {
        Self {
            name: name.into(),
            file: tmp,
        }
    }

    pub fn try_clone(&self) -> Result<Self, io::Error> {
        Ok(Self {
            name: self.name.clone(),
            file: self.file.try_clone()?,
        })
    }

    pub fn get_size_md5(&self) -> Result<(u64, String), io::Error> {
        let mut file = self.file.try_clone()?;

        let size = file.metadata()?.len();

        let mut v_context = md5::Context::new();
        let mut v_buffer = [0; 8192];

        loop {
            let n = io::Read::read(&mut file, &mut v_buffer)?;
            if n == 0 {
                break;
            }
            v_context.consume(&v_buffer[..n]);
        }

        let md5_video = v_context.finalize();
        let hex_video = hex::encode(md5_video.as_ref());

        Ok((size, hex_video))
    }

    pub fn size(&self) -> Result<u64, io::Error> {
        let meta = self.file.metadata()?;
        Ok(meta.len())
    }
}

impl PartialEq for TempFileReopen {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl Eq for TempFileReopen {}
impl Hash for TempFileReopen {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

#[derive(Debug)]
pub struct DownloadFilePending {
    pub tmp: TempFilePending,
    pub url: Url,
}

impl DownloadFilePending {
    #[track_caller]
    pub fn new<T: IntoUrl>(url: T, tmp: TempFilePending) -> Result<Self, DownloadFileError> {
        Ok(Self {
            url: url.into_url().map_err(|err| DownloadFileError {
                name: "url".into(),
                spawn: Location::caller(),
                error: err.into(),
            })?,
            tmp,
        })
    }

    #[track_caller]
    pub fn from_tmp_url<T: IntoUrl>(
        url: T,
        tmp: TempFilePending,
    ) -> Result<Self, DownloadFileError> {
        let url = url.into_url().map_err(|err| DownloadFileError {
            name: "url".into(),
            spawn: Location::caller(),
            error: err.into(),
        })?;

        Ok(Self { tmp, url })
    }

    pub fn new_with_bg<T: IntoUrl>(url: T, tmp: TempFilePending) -> Result<Self, reqwest::Error> {
        Ok(Self {
            tmp,
            url: url.into_url()?,
        })
    }

    pub async fn into_response(self) -> Result<DownloadFileResponse, DownloadFileError> {
        let Self { tmp, url } = self;

        let response =
            DownloadFileResponse::from_response(Self::response(url.as_str()).await?, tmp);

        Ok(response)
    }

    pub async fn response<T: IntoUrl>(url: T) -> Result<Response, DownloadFileError> {
        // default client
        let client = BiliApi::client();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|err| DownloadFileError {
                name: "url".into(),
                spawn: Location::caller(),
                error: err.into(),
            })?;
        Ok(response)
    }

    pub fn spawn_handle<F, O, T>(self, func: F) -> tokio::task::JoinHandle<T>
    where
        F: FnOnce(Self) -> O + Send + 'static,
        O: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        tokio::spawn(async move { func(self).await })
    }
}

pub struct DownloadFileResponse {
    pub response: Response,
    pub tmp: TempFilePending,
    pub way: Arc<
        dyn Fn(
                Response,
                TempFilePending,
            ) -> BoxFuture<'static, Result<TempFilePending, DownloadFileError>>
            + Send
            + Sync
            + 'static,
    >,
}

impl Debug for DownloadFileResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DownloadFileResponse")
            .field("response", &self.response)
            .field("tmp", &self.tmp)
            .finish()
    }
}

impl DownloadFileResponse {
    pub fn map<F: FnOnce(&Self)>(self, func: F) -> Self {
        func(&self);
        self
    }

    pub fn try_headers_size(&self) -> Result<u64, String> {
        use reqwest::header::CONTENT_LENGTH;
        let hv = &self.response.headers()[CONTENT_LENGTH];
        let str = hv.to_str().map_err(|e| e.to_string())?;
        let size = str.parse::<u64>().map_err(|e| e.to_string());

        size
    }

    #[track_caller]
    pub fn from_response(response: Response, tmp: TempFilePending) -> Self {
        Self {
            way: Arc::new(Self::default_chunk),
            tmp,
            response,
        }
    }

    pub async fn from_url<T: IntoUrl>(
        url: T,
        tmp: TempFilePending,
    ) -> Result<Self, DownloadFileError> {
        let response = DownloadFilePending::response(url).await?;

        Ok(Self {
            way: Arc::new(Self::default_chunk),
            tmp,
            response,
        })
    }

    pub fn response(&self) -> &Response {
        &self.response
    }

    pub fn with_mut<F: Fn(&mut Self)>(mut self, func: F) -> Self {
        func(&mut self);
        self
    }

    pub async fn with_download<
        F: FnOnce(Self) -> O,
        O: Future<Output = Result<Self, DownloadFileError>>,
    >(
        self,
        func: F,
    ) -> Result<Self, DownloadFileError> {
        func(self).await
    }

    pub fn set_download<
        F: Fn(
                Response,
                TempFilePending,
            ) -> BoxFuture<'static, Result<TempFilePending, DownloadFileError>>
            + 'static
            + Send
            + Sync,
    >(
        mut self,
        way: F,
    ) -> Self {
        self.way = Arc::new(way);
        self
    }

    pub async fn download(self) -> Result<TempFilePending, DownloadFileError> {
        let Self { tmp, response, way } = self;

        let tmp = way(response, tmp).await?;

        Ok(tmp)
    }

    #[track_caller]
    pub fn default_chunk(
        mut response: Response,
        mut tmp: TempFilePending,
    ) -> BoxFuture<'static, Result<TempFilePending, DownloadFileError>> {
        Box::pin(async move {
            const TIMEOVER: Duration = Duration::from_millis(3000);

            loop {
                let response = match tokio::time::timeout(TIMEOVER, response.chunk()).await {
                    Ok(response) => response,
                    Err(overtime) => {
                        return Err(DownloadFileError {
                            name: tmp.name.into(),
                            spawn: Location::caller(),
                            error: DownloadFileErrorKind::Overtime(overtime),
                        });
                    }
                };

                match response {
                    Ok(Some(chunk)) => {
                        tmp.tmp.write_all(&chunk).map_err(|err| DownloadFileError {
                            name: tmp.name.clone(),
                            spawn: Location::caller(),
                            error: DownloadFileErrorKind::IO(err),
                        })?;
                        tmp.tmp.flush().map_err(|err| DownloadFileError {
                            name: tmp.name.clone(),
                            spawn: Location::caller(),
                            error: DownloadFileErrorKind::IO(err),
                        })?;
                    }
                    Ok(None) => break,
                    Err(e) => {
                        return Err(DownloadFileError {
                            name: tmp.name,
                            spawn: Location::caller(),
                            error: DownloadFileErrorKind::Reqwest(e),
                        });
                    }
                }
            }

            Ok(tmp)
        })
    }

    pub async fn download_into_file_pendding(self) -> Result<TempFilePending, DownloadFileError> {
        self.download().await
    }

    pub fn spawn_handle<F, O, T>(self, func: F) -> tokio::task::JoinHandle<T>
    where
        F: FnOnce(Self) -> O + Send + 'static,
        O: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        tokio::spawn(async move { func(self).await })
    }
}

#[derive(Debug)]
pub struct DownloadFileError {
    pub name: Cow<'static, str>,
    pub spawn: &'static Location<'static>,
    pub error: DownloadFileErrorKind,
}

impl DownloadFileError {
    #[track_caller]
    pub fn new(name: Cow<'static, str>, error: DownloadFileErrorKind) -> Self {
        Self {
            name,
            spawn: Location::caller(),
            error,
        }
    }
}

impl std::error::Error for DownloadFileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.error {
            DownloadFileErrorKind::Reqwest(e) => Some(e),
            DownloadFileErrorKind::IO(e) => Some(e),
            DownloadFileErrorKind::SystemTime(e) => Some(e),
            DownloadFileErrorKind::ApiReq(e) => Some(e),
            DownloadFileErrorKind::Db(e) => Some(e),
            DownloadFileErrorKind::Serialize(e) => Some(e),
            DownloadFileErrorKind::MediaPage => None,
            DownloadFileErrorKind::Overtime(e) => Some(e),
        }
    }
}

#[derive(Debug)]
pub enum DownloadFileErrorKind {
    Reqwest(reqwest::Error),
    IO(io::Error),
    SystemTime(std::time::SystemTimeError),
    ApiReq(api_req::error::ApiErr),
    Db(sea_orm::DbErr),
    Serialize(serde_json::Error),
    MediaPage,
    Overtime(Elapsed),
}

impl std::fmt::Display for DownloadFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut print_err = |mid: &str, err: &dyn Debug| {
            write!(
                f,
                "Name: {:?},{} error:{:?}, caller:{:?}",
                self.name, mid, err, self.spawn
            )
        };
        match &self.error {
            DownloadFileErrorKind::Reqwest(error) => print_err("Reqwest", error),
            DownloadFileErrorKind::IO(error) => print_err("IO", error),
            DownloadFileErrorKind::SystemTime(error) => print_err("SystemTime", error),
            DownloadFileErrorKind::ApiReq(error) => print_err("ApiReq", error),
            DownloadFileErrorKind::Db(error) => print_err("Db", error),
            DownloadFileErrorKind::Serialize(error) => print_err("Serialize", error),
            DownloadFileErrorKind::MediaPage => {
                write!(
                    f,
                    "Name: {:?},media page error, maybe media not exist, caller:{:?}",
                    self.name, self.spawn
                )
            }
            DownloadFileErrorKind::Overtime(error) => print_err("download overtime", error),
        }
    }
}

impl From<io::Error> for DownloadFileErrorKind {
    fn from(error: io::Error) -> Self {
        Self::IO(error)
    }
}

impl From<reqwest::Error> for DownloadFileErrorKind {
    fn from(error: reqwest::Error) -> Self {
        Self::Reqwest(error)
    }
}

impl From<std::time::SystemTimeError> for DownloadFileErrorKind {
    fn from(error: std::time::SystemTimeError) -> Self {
        Self::SystemTime(error)
    }
}

impl From<api_req::error::ApiErr> for DownloadFileErrorKind {
    fn from(error: api_req::error::ApiErr) -> Self {
        Self::ApiReq(error)
    }
}

impl From<sea_orm::DbErr> for DownloadFileErrorKind {
    fn from(error: sea_orm::DbErr) -> Self {
        Self::Db(error)
    }
}

impl From<serde_json::Error> for DownloadFileErrorKind {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialize(error)
    }
}

#[track_caller]
pub fn danmuku_xml_download(response: DownloadFileResponse) -> DownloadFileResponse {
    response.set_download(move |mut response, mut pending| {
        Box::pin(async move {
            let mut vec = BytesMut::new();
            loop {
                match response.chunk().await {
                    Ok(Some(chunk)) => {
                        vec.extend_from_slice(&chunk);
                    }
                    Ok(None) => break,
                    Err(e) => {
                        return Err(DownloadFileError {
                            spawn: Location::caller(),
                            error: DownloadFileErrorKind::Reqwest(e),
                            name: DANMAKU_XML.into(),
                        });
                    }
                }
            }

            // 解压 deflate 数据
            let mut decoder = flate2::bufread::DeflateDecoder::new(&vec[..]);
            let mut decompressed = Vec::new();
            use std::io::Read;

            decoder
                .read_to_end(&mut decompressed)
                .map_err(|err| DownloadFileError {
                    spawn: Location::caller(),
                    error: DownloadFileErrorKind::IO(err),
                    name: DANMAKU_XML.into(),
                })?;

            pending
                .tmp
                .write_all(&decompressed)
                .map_err(|err| DownloadFileError {
                    spawn: Location::caller(),
                    error: DownloadFileErrorKind::IO(err),
                    name: DANMAKU_XML.into(),
                })?;

            Ok(pending)
        })
    })
}

const BAR_TEMPLATE: &str = "{msg} {spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})";

pub fn progress_bar(len: u64, bars: &MultiProgress, part: &str) -> ProgressBar {
    let bar = ProgressBar::new(len);
    bars.add(bar.clone());
    bar.set_message(format!("{:<10}", part));
    bar.set_style(
        ProgressStyle::with_template(BAR_TEMPLATE)
            .unwrap()
            .progress_chars("#>-"),
    );

    bar
}

/// 在现有 way 之上叠加进度条：每个 chunk 落盘后 inc。
/// 替换掉原来的 default_chunk_with_pg。
pub fn with_progress(response: DownloadFileResponse, pb: ProgressBar) -> DownloadFileResponse {
    response.set_download(move |mut response, mut tmp| {
        let pb = pb.clone();
        Box::pin(async move {
            const TIMEOVER: Duration = Duration::from_secs(10);

            loop {
                let chunk = match tokio::time::timeout(TIMEOVER, response.chunk()).await {
                    Ok(Ok(Some(c))) => c,
                    Ok(Ok(None)) => break,
                    Ok(Err(e)) => {
                        return Err(DownloadFileError {
                            name: tmp.name.clone(),
                            spawn: Location::caller(),
                            error: DownloadFileErrorKind::Reqwest(e),
                        });
                    }
                    Err(overtime) => {
                        return Err(DownloadFileError {
                            name: tmp.name.clone(),
                            spawn: Location::caller(),
                            error: DownloadFileErrorKind::Overtime(overtime),
                        });
                    }
                };

                tmp.tmp.write_all(&chunk).map_err(|e| DownloadFileError {
                    name: tmp.name.clone(),
                    spawn: Location::caller(),
                    error: DownloadFileErrorKind::IO(e),
                })?;
                tmp.tmp.flush().map_err(|e| DownloadFileError {
                    name: tmp.name.clone(),
                    spawn: Location::caller(),
                    error: DownloadFileErrorKind::IO(e),
                })?;

                pb.inc(chunk.len() as u64); // ← 核心
            }

            Ok(tmp)
        })
    })
}

pub type DownloadTotalBytes = Arc<AtomicU64>;

pub async fn new_download<U: IntoUrl, S: Into<Cow<'static, str>>, P: AsRef<Path>>(
    filename: S,
    url: U,
    tmp_path: P,
    target: impl IntoIterator<Item = PathBuf>,
    req_timeout: Duration,
) -> Result<DownloadFileResponse, DownloadFileError> {
    let filename = filename.into();
    let pending = DownloadFilePending::from_tmp_url(
        url,
        TempFilePending::new(
            filename.clone(),
            NamedTempFile::new_in(tmp_path).unwrap(),
            target,
        ),
    )?;

    tokio::time::timeout(req_timeout, pending.into_response())
        .await
        .map_err(|err| DownloadFileError::new(filename, DownloadFileErrorKind::Overtime(err)))?
}

pub const VIDEO: &str = "video.m4s";
pub const AUDIO: &str = "audio.m4s";
pub const DANMAKU: &str = "danmaku.pb";
pub const DANMAKU_XML: &str = "danmaku.xml";
pub const INDEX_JSON: &str = "index.json";
pub const ENTRY_JSON: &str = "entry.json";
pub const COVER_JPG: &str = "cover.jpg";

pub async fn upper_downloads<P: AsRef<Path>>(
    fetch_media: &FetchMedia,
    tmp_path: P,
    cookies: &str,
    upper_folders: Vec<PathBuf>,
) -> Vec<Result<TempFilePending, DownloadFileError>> {
    add_cookie_jar(parse_cookies(cookies));

    let downloadlists = [
        new_download(
            DANMAKU,
            &format!(
                "https://api.bilibili.com/x/v2/dm/web/seg.so?type=1&oid={}&segment_index=1",
                fetch_media.0.cid
            ),
            tmp_path.as_ref(),
            upper_folders.clone(),
            Duration::from_millis(3000),
        )
        .await,
        new_download(
            DANMAKU_XML,
            &format!(
                "https://api.bilibili.com/x/v1/dm/list.so?oid={}",
                fetch_media.0.cid
            ),
            tmp_path.as_ref(),
            upper_folders.clone(),
            Duration::from_millis(3000),
        )
        .await
        .map(|response| danmuku_xml_download(response)),
        new_download(
            COVER_JPG,
            fetch_media.0.pic.clone(),
            tmp_path.as_ref(),
            upper_folders.clone(),
            Duration::from_millis(3000),
        )
        .await,
    ];

    futures::future::join_all(
        downloadlists
            .into_iter()
            .map(|response| async move { response?.download().await }),
    )
    .await
}

pub async fn media_downloads<P: AsRef<Path>>(
    entry_json: &mut EntryOuput,
    index_video: &mut IndexVideo,
    index_audio: &mut IndexAudio,
    v: Option<&Video>,
    a: Option<&Audio>,
    pb: ProgressBar,
    total_bytes: Arc<AtomicU64>,
    tmp_path: P,
    cookies: &str,
    media_folders: Vec<PathBuf>,
) -> Vec<Result<TempFilePending, DownloadFileError>> {
    add_cookie_jar(parse_cookies(cookies));

    let mut downloadlists = [const { None }; 2];

    if let Some(video) = v {
        downloadlists[0] = Some(
            new_download(
                VIDEO,
                video.base_url.clone(),
                tmp_path.as_ref(),
                media_folders.clone(),
                Duration::from_millis(3000),
            )
            .await
            .map(|response| {
                let size = response.try_headers_size().unwrap_or_else(|err| {
                    println!("{:?}: get headers size fail: {:?}", VIDEO, err);
                    0
                });
                total_bytes.fetch_add(size, std::sync::atomic::Ordering::Relaxed);
                pb.inc_length(size);
                with_progress(response, pb.clone())
            }),
        );
        entry_json.update_video(video);
        index_video.update_video(video);
    };

    if let Some(audio) = a {
        downloadlists[1] = Some(
            new_download(
                AUDIO,
                audio.base_url.clone(),
                tmp_path.as_ref(),
                media_folders.clone(),
                Duration::from_millis(3000),
            )
            .await
            .map(|response| {
                let size = response.try_headers_size().unwrap_or_else(|err| {
                    println!("{:?}: get headers size fail: {:?}", VIDEO, err);
                    0
                });
                total_bytes.fetch_add(size, std::sync::atomic::Ordering::Relaxed);
                pb.inc_length(size);
                with_progress(response, pb.clone())
            }),
        );

        entry_json.update_audio(audio);
        index_audio.update_audio(audio);
        index_video.update_audio_id(audio.id);
    };

    futures::future::join_all(
        downloadlists
            .into_iter()
            .flatten()
            .map(|response| async move { response?.download().await }),
    )
    .await
}
