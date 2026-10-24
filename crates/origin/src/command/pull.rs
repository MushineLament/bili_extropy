use std::{
    fs::{self, File},
    io::{self, Seek, Write},
    path::Path,
    sync::{Arc, atomic::AtomicU64},
    time::Duration,
};

use api_req::ApiCaller;
use clap::{Arg, ArgMatches, Command};
use indicatif::{MultiProgress, ProgressDrawTarget};
use tempfile::NamedTempFile;

use crate::{
    RUNTIMER,
    api::BiliApi,
    command::{account::ACTIVE_ACCOUNT, status::STATUS},
    components::{
        download::{
            AUDIO, DownloadFileError, DownloadFileErrorKind, ENTRY_JSON, INDEX_JSON,
            TempFilePending, VIDEO, media_downloads, progress_bar, upper_downloads,
        },
        fetch::{FetchMedia, MediaInputId},
    },
    db::DB,
    entity::media::Page,
    output::{EntryOuput, IndexAudio, IndexOuput, IndexVideo},
    payload::DashPayload,
    response::DashResp,
    table::{ToTable, ToTableRecord},
};

pub fn command_pull() -> Command {
    Command::new("pull")
        .about("Pull fetched medias [alias: p]")
        .aliases(["p"])
        .arg_required_else_help(true)
        .subcommands([
            Command::new("status").about("Create Active Download Folders"),
            Command::new("media")
                .about("Pull media infomation")
                .args([Arg::new("media")
                    .help("Media BvId or Cid")
                    .value_name("BVID")
                    .required(true)]),
        ])
}

pub fn execute_pull(arg: &ArgMatches) {
    match arg.subcommand() {
        Some(("status", _)) => pull_status(),
        Some(("media", subarg)) => pull_media(subarg),

        _ => unreachable!(),
    }
}

pub fn pull_media(subarg: &ArgMatches) {
    pub const TEMP_DOWNLOAD_FOLDER: &str = ".temp";
    const TMP_PATH: &str = TEMP_DOWNLOAD_FOLDER;

    let Some(media_id) = subarg.get_one::<String>("media") else {
        println!("Not a BvId or Cid");
        return;
    };

    let inputid = MediaInputId::from_bvid_or_aid(media_id);
    let binding = STATUS;
    let status = binding.0.as_slice();

    let cookies = ACTIVE_ACCOUNT.0.cookies.clone();
    let db = DB.clone();
    let fetch_media = RUNTIMER.block_on(async {
        tokio::time::timeout(
            Duration::from_secs(3),
            FetchMedia::async_fetch(&db, inputid, &cookies),
        )
        .await
    });

    let fetch_media = match fetch_media {
        Ok(result) => match result {
            Ok(result) => result,
            Err(err) => {
                return println!("fetch media<{}> error: {:?}", media_id, err);
            }
        },
        Err(_) => {
            println!("fetch media timeout");
            return;
        }
    };

    let system_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("compute system's time error");

    let fetch_dash = RUNTIMER.block_on(async {
        tokio::time::timeout(
            Duration::from_secs(3),
            DashPayload::new(fetch_media.0.aid, fetch_media.0.cid, system_time.as_secs()),
        )
        .await
    });

    let fetch_dash = match fetch_dash {
        Ok(result) => match result {
            Ok(result) => result,
            Err(err) => return println!("fetch media dash payload error: {:?}", err),
        },
        Err(_) => return println!("fetch media dash payload timeout"),
    };

    let fetch_dashresp = RUNTIMER.block_on(async {
        tokio::time::timeout(Duration::from_secs(3), BiliApi::request(fetch_dash)).await
    });

    let fetch_dashresp: DashResp = match fetch_dashresp {
        Ok(result) => match result {
            Ok(result) => result,
            Err(err) => return println!("fetch media dashresp error: {:?}", err),
        },
        Err(_) => return println!("fetch media dashresp timeout"),
    };

    let mut entry_json = EntryOuput::default();
    entry_json.time_create_stamp = system_time.as_millis();

    let mut index_output = IndexOuput::default();

    // todo! 输出视频名字
    println!("start download media<{}>", fetch_media.0.title);

    let mediaaid = fetch_media.0.aid.to_string();
    let uppercid = format!("c_{}", fetch_media.0.cid.to_string());

    if fetch_media.0.pages.is_empty() {
        println!("download media<{}>'s pages is empty", fetch_media.0.bvid);
        return;
    }

    let mut upper_downloadlists = vec![];

    let upper_folders = status
        .into_iter()
        .map(|model| Path::new(&model.path).join(&model.name))
        .map(|path| path.join(mediaaid.as_str()))
        .map(|path| path.join(uppercid.as_str()))
        .collect::<Vec<_>>();

    let only1p = fetch_media.0.pages.len() == 1;
    for (
        _page_index,
        Page {
            cid: pagecid,
            page,
            part,
        },
    ) in fetch_media.0.pages.iter().enumerate()
    {
        entry_json.update_page(*pagecid, *page);
        entry_json.total_time_milli = fetch_dashresp.data.timelength;

        let mut index_video = IndexVideo::default();
        let mut index_audio = IndexAudio::default();

        let _filename = if only1p {
            format!("{}-{}", fetch_media.0.aid, fetch_media.0.title)
        } else {
            format!(
                "{}-{}({page})-{part}",
                fetch_media.0.aid, fetch_media.0.title
            )
        };

        let mut video = fetch_dashresp.data.dash.video.iter();
        let mut audio = fetch_dashresp.data.dash.audio.iter();

        let (v, a) = (video.next(), audio.next());

        if v.is_none() && a.is_none() {
            // TODO! need more detailed error infomation.
            println!("media<{}> dash not any video or audio", fetch_media.0.bvid);
            continue;
        }

        let bars = MultiProgress::with_draw_target(ProgressDrawTarget::stderr());

        let pb = progress_bar(0, &bars, &part);

        let total_bytes = Arc::new(AtomicU64::new(0));

        let folder_video_id = v.as_ref().map(|v| v.id).unwrap_or_default().to_string();

        let media_folders = upper_folders
            .iter()
            .map(|path| path.join(&folder_video_id))
            .collect::<Vec<_>>();

        let media_downloadlists = RUNTIMER.block_on(media_downloads(
            &mut entry_json,
            &mut index_video,
            &mut index_audio,
            v,
            a,
            pb.clone(),
            total_bytes.clone(),
            TMP_PATH,
            &cookies,
            media_folders.clone(),
        ));

        for result in media_downloadlists {
            let Ok(response) = result else {
                println!("download media<{}> error: {:?}", fetch_media.0.bvid, result);
                continue;
            };

            let (size, md5) = response.get_size_md5().unwrap_or_else(|err: io::Error| {
                println!(
                    "download media<{}> get size md5 error: {:?}",
                    fetch_media.0.bvid, err
                );
                (0, "get md5 error".to_string())
            });

            if response.name == VIDEO {
                index_video.update_md5_size(md5, size);
            } else if response.name == AUDIO {
                index_audio.update_md5_size(md5, size);
            }

            for target in build(response) {
                if let Err(err) = target {
                    println!(
                        "download media<{}> build error: {:?}",
                        fetch_media.0.bvid, err
                    );
                }
            }
        }

        entry_json.total_bytes = pb.position();
        entry_json.page_data.from = "vupload".to_string();
        entry_json.is_completed = true;
        entry_json.downloaded_bytes = total_bytes.load(std::sync::atomic::Ordering::Relaxed);

        index_output.video.push(index_video);
        index_output.audio.push(index_audio);

        let index_result = (|| -> Result<TempFilePending, DownloadFileError> {
            let mut tmp = NamedTempFile::new_in(TMP_PATH).map_err(|e| {
                DownloadFileError::new(INDEX_JSON.into(), DownloadFileErrorKind::IO(e))
            })?;

            serde_json::to_writer_pretty(&mut tmp, &index_output).map_err(|e| {
                DownloadFileError::new(INDEX_JSON.into(), DownloadFileErrorKind::Serialize(e))
            })?;

            tmp.flush().map_err(|e| {
                DownloadFileError::new(INDEX_JSON.into(), DownloadFileErrorKind::IO(e))
            })?;

            Ok(TempFilePending::new(INDEX_JSON, tmp, media_folders))
        })();

        match index_result {
            Ok(index) => {
                for target in build(index) {
                    if let Err(err) = target {
                        println!(
                            "download media<{}> build error: {:?}",
                            fetch_media.0.bvid, err
                        );
                    }
                }
            }
            Err(err) => println!(
                "download media<{}> build error: {:?}",
                fetch_media.0.bvid, err
            ),
        }

        // 默认取最高画质进行缓存
        break;
    }

    entry_json.update_media(&fetch_media.0);
    entry_json.update_owner(&fetch_media.0.owner);
    entry_json.time_update_stamp = system_time.as_millis();

    let entry_result = (|| -> Result<TempFilePending, DownloadFileError> {
        let mut tmp = NamedTempFile::new_in(TMP_PATH)
            .map_err(|e| DownloadFileError::new(ENTRY_JSON.into(), DownloadFileErrorKind::IO(e)))?;

        serde_json::to_writer_pretty(&mut tmp, &entry_json).map_err(|e| {
            DownloadFileError::new(ENTRY_JSON.into(), DownloadFileErrorKind::Serialize(e))
        })?;

        tmp.flush()
            .map_err(|e| DownloadFileError::new(ENTRY_JSON.into(), DownloadFileErrorKind::IO(e)))?;

        Ok(TempFilePending::new(ENTRY_JSON, tmp, upper_folders.clone()))
    })();

    upper_downloadlists.push(entry_result);

    upper_downloadlists.extend(RUNTIMER.block_on(upper_downloads(
        &fetch_media,
        TMP_PATH,
        &cookies,
        upper_folders.clone(),
    )));

    for result in upper_downloadlists {
        let Ok(response) = result else {
            println!("download media<{}> error: {:?}", fetch_media.0.bvid, result);
            continue;
        };

        for target in build(response) {
            if let Err(err) = target {
                println!(
                    "download media<{}> build error: {:?}",
                    fetch_media.0.bvid, err
                );
            }
        }
    }
}

// 后续计划，实现错误收集，以便支持自行处理
// 将 NamedTempFile 移动到指定目录下，并重命名指定文件名
pub fn build(pendings: TempFilePending) -> Vec<Result<u64, io::Error>> {
    let mut vec = vec![];

    let TempFilePending {
        name,
        mut tmp,
        target,
    } = pendings;

    for folder in target {
        if !folder.exists() {
            if let Err(err) = fs::create_dir_all(&folder) {
                vec.push(Err(err));
                continue;
            };
        }

        if !folder.is_dir() {
            vec.push(Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                "copy into folder error",
            )));
            continue;
        }

        if let Err(e) = tmp.seek(io::SeekFrom::Start(0)) {
            vec.push(Err(io::Error::new(
                io::ErrorKind::NotSeekable,
                format!("Failed to seek file {:?} to start: {}", name, e),
            )));
            continue;
        }

        let target_path = folder.join(name.as_ref());

        if target_path.exists() {
            vec.push(Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("target file exists error:{:?}", target_path),
            )));
            continue;
        }

        let mut target_file = match File::create(&target_path) {
            Ok(file) => file,
            Err(err) => {
                vec.push(Err(err));
                continue;
            }
        };

        vec.push(io::copy(&mut tmp, &mut target_file));
    }

    vec
}

pub fn pull_status() {
    struct PullFolder {
        id: i64,
        message: String,
    }
    impl ToTableRecord<2> for PullFolder {
        fn to_record(&self) -> [std::borrow::Cow<'_, str>; 2] {
            [self.id.to_string().into(), self.message.to_string().into()]
        }
    }
    let table = STATUS
        .0
        .iter()
        .map(|model| {
            let path = Path::new(&model.path).join(&model.name);
            if !path.exists()
                && let Err(err) = fs::create_dir_all(&path)
            {
                return PullFolder {
                    id: model.id,
                    message: err.to_string(),
                };
            }

            if !path.is_dir() {
                return PullFolder {
                    id: model.id,
                    message: "not a directory".to_string(),
                };
            }

            PullFolder {
                id: model.id,
                message: format!(
                    "{}: {}",
                    model.state,
                    if model.state.as_str() == "Active" {
                        "Exist"
                    } else {
                        "Ignore"
                    }
                    .to_string()
                ),
            }
        })
        .collect::<Vec<_>>();

    let table = table.iter().table_head(["id", "folder error"]);

    println!("\n{}\nrows: {}", table, table.count_rows() - 1);
}

// pub fn spawn_list_task(
//     mut commands: Commands,
//     db: Res<Db>,
//     mut runtimer: ResMut<TokioTasksRuntime>,
//     mut console_message: MessageReader<ConsoleTrims>,
// ) {
//     for message in console_message.read() {
//         let _db = db.clone();
//         let ConsoleTrims { args, argv: _ } = message;

//         if !args.get(1).is_some_and(|list| list.eq("pull")) {
//             continue;
//         }

//         match args.get(PULL_COMMAND_INDEX).map(String::as_str) {
//             Some(unkown) => {
//                 error!("not has this command: {:?}", unkown);
//             }

//             None => {
//                 // 输出help
//                 commands.spawn((
//                     LoadDownloadtaskMedias::new(db.clone(), runtimer.as_mut()),
//                     PullTask,
//                 ));
//             }
//         }
//     }
// }

// pub fn pull_downloadtask_medias(
//     mut commands: Commands,
//     query: Query<(&mut LoadDownloadtaskMedias, Entity), With<PullTask>>,
// ) {
//     for (mut result, entity) in query {
//         if !result.is_finished() {
//             continue;
//         }

//         match result.try_result() {
//             Ok(result) => {
//                 let mut hash: HashMap<MediaAid, DownloadtaskRelatedMediaPending> = HashMap::new();

//                 for model in result {
//                     match hash.entry(model.media_id) {
//                         Entry::Occupied(occupied) => {
//                             occupied.into_mut().taskid.push(model.task_id);
//                         }
//                         Entry::Vacant(vacant) => {
//                             vacant.insert(DownloadtaskRelatedMediaPending {
//                                 media_id: model.media_id,
//                                 taskid: vec![model.task_id],
//                             });
//                         }
//                     }
//                 }

//                 commands.spawn_batch(hash.into_values());
//             }
//             Err(error) => {
//                 error!("pull downloadtask's related medias error: {:?}", error);
//             }
//         }

//         commands.entity(entity).try_despawn();
//     }
// }
