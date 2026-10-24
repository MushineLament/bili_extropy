use std::sync::LazyLock;

use tokio::runtime::{Builder, Runtime};

pub mod command;
pub mod console;
pub mod db;
pub mod entity;
pub mod table;

pub mod api;
pub mod cookies;
pub mod output;
pub mod payload;
pub mod response;
pub mod state;
pub mod wbi;

pub mod components;

pub mod plugins;

pub mod version;

pub const RUNTIMER: LazyLock<Runtime> = LazyLock::new(|| {
    Builder::new_multi_thread()
        .worker_threads(8) // Worker 线程数
        .max_blocking_threads(32) // Blocking 线程池上限
        .thread_name("tokio-worker") // 便于调试
        .thread_stack_size(3 * 1024 * 1024) // 3MB 栈
        .enable_all() // 启用 IO、Timer 等驱动
        .build()
        .expect("待完善")
});
