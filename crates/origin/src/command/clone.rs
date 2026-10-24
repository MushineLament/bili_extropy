use std::{
    fs,
    io::{self, ErrorKind, Write},
    path::Path,
    sync::{Arc, atomic::AtomicU64},
    time::Duration,
};

use clap::{Arg, ArgAction, Command};
use indicatif::{MultiProgress, ProgressDrawTarget};
use tempfile::NamedTempFile;
use tracing::{error, info};

use crate::{
    command::HELP,
    components::{
        account::account::ActiveAccounts,
        download::{
            CopyFileEnd, CopyFileStart, CopyIntoFolders, DownloadDashIndex, DownloadDepend,
            DownloadFileAudio, DownloadFileDanmu, DownloadFileDanmuXml, DownloadFileErrorKind,
            DownloadFileIcon, DownloadFileVideo, DownloadMultiProgress, DownloadProgressBar,
            DownloadTaskStart, DownloadTotalBytes, FileDownloadEnd, FileDownloadStart,
            IndexOutputDepend, MediaBvidOrAid, MediaInputId, RelatedDownload, TEMP_DOWNLOAD_FOLDER,
            TempFilePending,
        },
        fetch::fetch::{FetchMedia, FetchMediaDashResp},
        handle::ECSHandleResult,
        status::status::ActiveStatus,
    },
    db::Db,
    entity::media::Page,
    output::{
        DenpendEntryOuput, EntryOuput, EntryOuputRelated, IndexAudio, IndexOuput, IndexVideo,
    },
};

pub fn command_fetch() -> Command {
    Command::new("clone")
                        .about("point clone metadata of following ups, fav sets, medias, ups [alias: f]")
                        .aliases(["cl"])
                        .args([Arg::new("prune")
                            .long("prune")
                            .short('p')
                            .help("Prune the objs: remove unfaved sets, unfollowed ups and medias not belonging to active set or up")
                            .action(ArgAction::SetTrue)])
}

pub fn console_continue(
    mut message: MessageWriter<ConsoleContinue>,
    query: Query<(), With<RelatedDownload>>,
) {
    if query.is_empty() {
        return;
    }

    info!("当前下载数量:{}", query.count());

    message.write_default();
}

pub fn spawn_file_download(
    mut commands: Commands,
    mut runtimer: ResMut<TokioTasksRuntime>,
    active_account: ActiveAccounts,
    query_downloads: Query<(), With<DownloadDepend>>,
    mut entry_lists: Query<
        (
            &mut EntryOuput,
            &MediaInputId,
            &FetchMedia,
            &FetchMediaDashResp,
            Entity,
        ),
        (With<DownloadTaskStart>, Without<RelatedDownload>),
    >,
    active_status: Query<&ActiveStatus>,
) {
    if entry_lists.count() <= 0 {
        return;
    }

    if active_status.is_empty() {
        error!(
            "error:{:?}",
            DownloadFileErrorKind::Status(anyhow::anyhow!("Not any status error"),)
        );
        return;
    }

    let count = query_downloads.count();

    let Some(sub) = 4usize.checked_sub(count).filter(|sub| *sub != 0) else {
        return;
    };

    let Some(_cookies) = active_account.get_first_cookies() else {
        error!("not any active account, fetch media failed");
        return;
    };

    let pendings = (0..sub).zip(entry_lists.iter_mut());

    let system_time = std::time::SystemTime::now();

    let time_create_stamp = system_time
        .duration_since(std::time::UNIX_EPOCH)
        .expect("get compute system's time error");

    const TMP_PATH: &str = TEMP_DOWNLOAD_FOLDER;

    if !Path::new(TMP_PATH).exists() {
        if let Err(error) = fs::create_dir_all(TMP_PATH) {
            error!("error:{:?}", error);
        }
    }

    if !Path::new(TMP_PATH).is_dir() {
        error!(
            "error:{:?}",
            DownloadFileErrorKind::IO(io::Error::new(
                ErrorKind::NotADirectory,
                "can't store tmp file"
            ),)
        );
        return;
    }

    let bars = DownloadMultiProgress(MultiProgress::with_draw_target(ProgressDrawTarget::stderr()));

    for (_index, (mut file_entry_json, input, media, resp, entry_entity)) in pendings {
        commands
            .entity(entry_entity)
            .remove::<DownloadTaskStart>()
            .insert(RelatedDownload::default());

        let (Ok(media), Ok(resp)) = (media.0.as_ref(), resp.0.as_ref()) else {
            error!(
                "input id error: {:?},media: {:?}, resp: {:?}",
                input, media, resp
            );
            continue;
        };

        // todo! 输出视频名字
        info!("spawn a download handle");

        let mediaaid = media.aid.to_string();
        let uppercid = format!("c_{}", media.cid.to_string());
        let folders = CopyIntoFolders(
            active_status
                .into_iter()
                .map(|path| path.legal_path())
                .map(|path| path.join(mediaaid.as_str()))
                .map(|path| path.join(uppercid.as_str()))
                .collect(),
        );

        if let Ok(task) =
            DownloadFileIcon::new(runtimer.as_mut(), media.pic.as_str(), TEMP_DOWNLOAD_FOLDER)
                .map_err(|err| error!("download media<{}> icon error:{:?}", media.bvid, err))
        {
            commands.spawn((DownloadDepend(entry_entity), task, folders.clone()));
        };

        if media.pages.is_empty() {
            error!("download media<{}>'s pages is empty", media.bvid);
            commands.entity(entry_entity).try_despawn();
            continue;
        }

        let only1p = media.pages.len() == 1;
        for (
            _page_index,
            Page {
                cid: pagecid,
                page,
                part,
            },
        ) in media.pages.iter().enumerate()
        {
            file_entry_json.update_page(*pagecid, *page);

            file_entry_json.total_time_milli = resp.data.timelength;

            let _filename = if only1p {
                format!("{}-{}", media.aid, media.title)
            } else {
                format!("{}-{}({page})-{part}", media.aid, media.title)
            };

            // 0 mean dash index
            const RESP_VIDEO_INDEX: usize = 0;
            let mut video = resp.data.dash.video.iter();

            const RESP_AUDIO_INDEX: usize = 0;
            let mut audio = resp.data.dash.audio.iter();

            let (v, a) = (video.next(), audio.next());

            if v.is_none() && a.is_none() {
                // TODO! need more detailed error infomation.
                error!("media<{}> dash not any video or audio", media.bvid);
                commands.entity(entry_entity).try_despawn();
                continue;
            }

            let pb = DownloadProgressBar::default();
            pb.progress_bar(bars.0.clone(), &part);

            let total_bytes = DownloadTotalBytes(Arc::new(AtomicU64::new(0)));

            let folder_video_id = v.as_ref().map(|v| v.id).unwrap_or_default().to_string();

            let resolution_folders = CopyIntoFolders(
                folders
                    .iter()
                    .map(|path| path.join(folder_video_id.as_str()))
                    .collect::<Vec<_>>(),
            );

            let index_related = commands
                .spawn((
                    DenpendEntryOuput(entry_entity),
                    resolution_folders.clone(),
                    IndexVideo::default(),
                    pb.clone(),
                    total_bytes.clone(),
                ))
                .id();

            if let Some(video) = v
                && let Ok(task) = DownloadFileVideo::new_tracing(
                    runtimer.as_mut(),
                    pb.clone(),
                    total_bytes.clone(),
                    video.base_url.clone(),
                    TMP_PATH,
                )
                .map_err(|err| {
                    error!(
                        "spawn download media<{}>'s video task error:{:?}",
                        media.bvid, err
                    );
                })
            {
                Some(
                    commands
                        .spawn((
                            task,
                            resolution_folders.clone(),
                            DownloadDepend(entry_entity),
                            IndexOutputDepend(index_related),
                            DownloadDashIndex(RESP_VIDEO_INDEX),
                        ))
                        .id(),
                )
            } else {
                None
            };

            if let Some(audio) = a
                && let Ok(task) = DownloadFileAudio::new_tracing(
                    runtimer.as_mut(),
                    pb.clone(),
                    total_bytes.clone(),
                    audio.base_url.clone(),
                    TMP_PATH,
                )
                .map_err(|err| {
                    error!(
                        "spawn download media<{}>'s audio task error:{:?}",
                        media.bvid, err
                    );
                })
            {
                commands
                    .entity(index_related)
                    .insert((IndexAudio::default(),));

                commands.spawn((
                    task,
                    resolution_folders.clone(),
                    DownloadDepend(entry_entity),
                    IndexOutputDepend(index_related),
                    DownloadDashIndex(RESP_AUDIO_INDEX),
                ));
            };

            if let Ok(task) = DownloadFileDanmu::new(runtimer.as_mut(), media.cid, TMP_PATH)
                .map_err(|err| {
                    error!("download media<{}> danmu error:{:?}", media.bvid, err);
                })
            {
                commands.spawn((
                    task,
                    resolution_folders.clone(),
                    DownloadDepend(entry_entity),
                    IndexOutputDepend(index_related),
                ));
            }

            if let Ok(task) = DownloadFileDanmuXml::new(runtimer.as_mut(), media.cid, TMP_PATH)
                .map_err(|err| {
                    error!("download media<{}> danmuxml error:{:?}", media.bvid, err);
                })
            {
                commands.spawn((
                    task,
                    resolution_folders.clone(),
                    DownloadDepend(entry_entity),
                    IndexOutputDepend(index_related),
                ));
            }

            // 默认取最高画质进行缓存
            break;
        }

        let mut entry_output = EntryOuput::default();
        entry_output.time_create_stamp = time_create_stamp.as_millis();

        commands
            .entity(entry_entity)
            .insert((entry_output, folders, bars.clone()));
    }
}

pub fn update_index_by_video(
    mut query_index: Query<&mut IndexVideo>,
    query_resp: Query<&FetchMediaDashResp>,
    query_download: Query<
        (
            &mut FileDownloadStart,
            &DownloadDepend,
            &IndexOutputDepend,
            &DownloadDashIndex,
            Entity,
        ),
        (With<DownloadFileVideo>, Added<FileDownloadEnd>),
    >,
) {
    for (mut handle, depend, output, page_index, entity) in query_download {
        if !handle.is_finished() {
            continue;
        }

        let result = match handle.try_result() {
            Ok(handle) => handle,
            Err(err) => {
                error!("download video error: {:?} ,entity: {:?}", err, entity);
                continue;
            }
        };

        if let Ok(mut index) = query_index.get_mut(output.0) {
            if let Ok(FetchMediaDashResp(Ok(dash))) = query_resp.get(depend.0)
                && let Some(video) = dash.data.dash.video.get(page_index.0)
            {
                index.update_video(video);
            }

            if let Ok((size, md5)) = result.get_size_md5().map_err(|err| {
                error!(
                    "download video get size and md5,entity: {:?},error: {:?}",
                    entity, err
                );
            }) {
                index.update_md5_size(md5, size);
            };
        }
    }
}

pub fn update_index_by_audio(
    mut query_index: Query<(&mut IndexAudio, Option<&mut IndexVideo>)>,
    query_resp: Query<&FetchMediaDashResp>,
    query_audio: Query<
        (
            &mut FileDownloadStart,
            &DownloadDepend,
            &IndexOutputDepend,
            &DownloadDashIndex,
            Entity,
        ),
        (With<DownloadFileAudio>, Added<FileDownloadEnd>),
    >,
) {
    for (mut handle, depend, output, page_index, entity) in query_audio {
        if !handle.is_finished() {
            continue;
        }

        let result = match handle.try_result() {
            Ok(handle) => handle,
            Err(err) => {
                error!("download audio error: {:?} ,entity: {:?}", err, entity);
                continue;
            }
        };

        if let Ok((mut index_audio, index_video)) = query_index.get_mut(output.0) {
            if let Ok(FetchMediaDashResp(Ok(dash))) = query_resp.get(depend.0)
                && let Some(audio) = dash.data.dash.audio.get(page_index.0)
            {
                index_audio.update_audio(audio);
                if let Some(mut index_video) = index_video {
                    index_video.update_audio_id(audio.id);
                }
            }

            if let Ok((size, md5)) = result.get_size_md5().map_err(|err| {
                error!(
                    "download audio get size and md5,entity: {:?},error: {:?}",
                    entity, err
                );
            }) {
                index_audio.update_md5_size(md5, size);
            };
        }
    }
}

pub fn download_finished_then_copy(
    mut commands: Commands,
    query: Query<(&mut FileDownloadStart, Entity), Without<FileDownloadEnd>>,
) {
    for (mut handle, entity) in query {
        if !handle.is_finished() {
            continue;
        }

        let Ok(pending) = handle.try_result().map_err(|err| {
            error!("download audio error: {:?} ,entity: {:?}", err, entity);
        }) else {
            continue;
        };

        let Ok(reopen) = pending.as_reopon().map_err(|err| {
            error!("download audio error: {:?} ,entity: {:?}", err, entity);
        }) else {
            continue;
        };

        // if download finished,insert download end and start copy,consume CopyIntoFolders
        commands
            .entity(entity)
            .insert(FileDownloadEnd)
            .queue(|mut entity: EntityWorldMut| {
                if let Some(folder) = entity.take::<CopyIntoFolders>() {
                    let mut runtimer = entity.resource_mut::<TokioTasksRuntime>();
                    let copy = CopyFileStart::new(runtimer.as_mut(), folder.clone(), reopen);
                    entity.insert(copy);
                } else {
                    todo!("打印复制文件目录丢失");
                }
            });
    }
}

pub fn copy_file_finished_observer(
    mut commands: Commands,
    query: Query<(&mut CopyFileStart, Entity), Without<FileDownloadEnd>>,
) {
    for (mut handle, entity) in query {
        if !handle.is_finished() {
            continue;
        }

        let _ = handle.try_result();

        commands.entity(entity).insert(CopyFileEnd);
    }
}

/// when index video or audio changed,will update entryoutput
pub fn update_entry_ouput(
    mut entry_query: Query<(&mut EntryOuput, &FetchMedia, &FetchMediaDashResp)>,
    index_query: Query<
        (
            &DenpendEntryOuput,
            &DownloadDashIndex,
            Option<&DownloadProgressBar>,
            Option<&DownloadTotalBytes>,
            Has<IndexVideo>,
            Has<IndexAudio>,
        ),
        Or<(Changed<IndexVideo>, Changed<IndexAudio>)>,
    >,
) {
    let system_time = std::time::SystemTime::now();

    for (denpend_entry, resp_index, pb, total_bytes, is_video, is_audio) in index_query {
        let Ok((mut file_entry_json, media, media_info)) = entry_query.get_mut(denpend_entry.0)
        else {
            continue;
        };

        let (Ok(resp), Ok(media)) = (media_info.as_ref(), media.as_ref()) else {
            // todo!("输出错误");
            continue;
        };

        file_entry_json.update_media(media);
        file_entry_json.update_owner(&media.owner);

        if is_video && let Some(video) = resp.data.dash.video.get(resp_index.0) {
            file_entry_json.update_video(video);
        }

        if is_audio && let Some(audio) = resp.data.dash.audio.get(resp_index.0) {
            file_entry_json.update_audio(audio);
        }

        if let Some(pb) = pb {
            file_entry_json.total_bytes = pb.position();
        }
        file_entry_json.page_data.from = "vupload".to_string();
        file_entry_json.is_completed = true;

        if let Some(total_bytes) = total_bytes {
            file_entry_json.downloaded_bytes =
                total_bytes.load(std::sync::atomic::Ordering::Relaxed);
        }

        let duration_file_update = system_time
            .duration_since(std::time::UNIX_EPOCH)
            .expect("get compute system's time error");
        file_entry_json.time_update_stamp = duration_file_update.as_millis();
    }
}

/// entry.json is lastest copy into folder,copy finished mean download finished
pub fn entry_and_index_finished_copy_start(
    mut commands: Commands,
    query_entry: Query<(
        &RelatedDownload,
        &mut EntryOuput,
        &CopyIntoFolders,
        &EntryOuputRelated,
        Entity,
    )>,

    query_index: Query<
        (Option<&IndexVideo>, Option<&IndexAudio>),
        Or<(With<IndexVideo>, With<IndexAudio>)>,
    >,
    query_download: Query<(), With<FileDownloadEnd>>,
) {
    const TMP_PATH: &str = TEMP_DOWNLOAD_FOLDER;

    'entry: for (related_downloads, _entry, _intofolder, related_indexs, entity) in query_entry {
        if related_indexs.is_empty() {
            todo!("输出index缺失，也有可能是因为 page media 为空，但是却开启下载等边界情况");
            continue;
        }

        // only related downloads all is finished,start finished entry output.
        if related_downloads
            .iter()
            .any(|e| !query_download.contains(e))
        {
            continue;
        }

        let Ok(file_entry) = NamedTempFile::new_in(TMP_PATH) else {
            error!("create entry.json temp file error");
            continue;
        };

        // todo! 按理来说，需要检测是否有update index的情况
        for index_entity in related_indexs.iter() {
            let Ok(_reopon) = query_index.get(index_entity) else {
                todo!("错误输出");
                continue;
            };
            commands
                .entity(index_entity)
                .queue(|mut entity: EntityWorldMut| {
                    let Some(folders) = entity.take::<CopyIntoFolders>() else {
                        return;
                    };

                    let mut index = IndexOuput::default();

                    if let Some(video) = entity.take::<IndexVideo>() {
                        index.video.push(video);
                    }
                    if let Some(audio) = entity.take::<IndexAudio>() {
                        index.audio.push(audio);
                    }

                    let runtimer = entity.resource_mut::<TokioTasksRuntime>();

                    let handle = async move {
                        NamedTempFile::new_in(TMP_PATH)
                            .map(|file_index| TempFilePending::new("index.json", file_index))
                            .and_then(|file_index| {
                                let index_writer = io::BufWriter::new(&file_index.tmp);
                                serde_json::to_writer_pretty(index_writer, &index)?;
                                Ok(file_index)
                            })
                            .and_then(|file_index| {
                                let reopobn = file_index.to_reopon()?;
                                Ok(folders.build(reopobn))
                            })
                    };

                    let copy = CopyFileStart(ECSHandleResult::new(
                        runtimer.spawn_background_task(|_| handle),
                    ));

                    entity.insert(copy);
                });
        }

        commands.entity(entity).queue(|mut entity: EntityWorldMut| {
            let Some(output) = entity.take::<EntryOuput>() else {
                error!("get empty, output entry entity:{:?}", entity.id());
                return;
            };

            let Some(intos) = entity.take::<CopyIntoFolders>() else {
                error!(
                    "get empty, output entry into folder, entity:{:?}",
                    entity.id()
                );
                return;
            };

            let handle = async move {
                let mut output = output;
                let mut file_entry = TempFilePending::new("entry.json", file_entry);

                let entry_writer = io::BufWriter::new(&file_entry.tmp);
                if let Err(err) = serde_json::to_writer_pretty(entry_writer, &mut output) {
                    error!("write bytes into entry.json error:{:?}", err);
                }; // 使用 pretty 格式化输出;

                file_entry.tmp.flush()?;

                let reopobn = file_entry.to_reopon()?;

                Ok(intos.build(reopobn))
            };

            let resource = entity.resource_mut::<TokioTasksRuntime>();

            let copystart = CopyFileStart(ECSHandleResult::new(
                resource.spawn_background_task(|_ctx| handle),
            ));

            entity.insert(copystart);
        });
    }
}

pub fn entry_output_copy_end(
    mut commands: Commands,
    query_entry: Query<
        (&mut CopyFileStart, &EntryOuputRelated, &FetchMedia, Entity),
        With<CopyFileEnd>,
    >,
) {
    for (mut handle, entry_relateds, media, entity) in query_entry {
        let Ok(pending) = handle.try_result().map_err(|err| {
            error!("copy file error: {:?} ,entity: {:?}", err, entity);
        }) else {
            continue;
        };

        if let FetchMedia(Ok(media)) = media {
            info!("media<{}>: `{}`, 复制到目录完毕", media.bvid, media.title);
        };

        for err in pending.iter().map(|e| e.as_ref().err()).flatten() {
            error!("copy file into folder: {}", err);
        }

        for entry_related in entry_relateds.iter() {
            commands
                .entity(entry_related)
                .despawn_related::<RelatedDownload>();
        }

        commands
            .entity(entity)
            .despawn_related::<EntryOuputRelated>()
            .try_despawn();
    }
}
