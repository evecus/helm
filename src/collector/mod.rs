//! 采集模块入口：后台缓存（systemd / docker）与统一快照。
//! 对应 Go 版 internal/collector/cache.go。

pub mod docker;
pub mod process;
pub mod systemd;
pub mod system;

use crate::state::AppState;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub use system::MetricsSnapshot;

// ── 通用缓存 ──────────────────────────────────────────────────────

pub struct CacheInner<T> {
    pub data: Vec<T>,
    pub ready: bool,
    pub updated_at: Instant,
}

pub struct Cache<T> {
    pub inner: Mutex<CacheInner<T>>,
}

impl<T> Cache<T> {
    pub fn new() -> Self {
        Cache {
            inner: Mutex::new(CacheInner {
                data: Vec::new(),
                ready: false,
                updated_at: Instant::now(),
            }),
        }
    }
}

impl<T> Default for Cache<T> {
    fn default() -> Self {
        Self::new()
    }
}

fn lock<'a, T>(c: &'a Cache<T>) -> std::sync::MutexGuard<'a, CacheInner<T>> {
    c.inner.lock().unwrap_or_else(|p| p.into_inner())
}

// ── Systemd 缓存 ──────────────────────────────────────────────────

/// 对应 GetServicesFromCache：未就绪时同步采集一次，返回前排序。
pub fn get_services_from_cache(
    state: &Arc<AppState>,
    sort_by: &str,
    sort_dir: &str,
) -> Result<Vec<systemd::SystemdService>, String> {
    let mut snapshot: Option<Vec<systemd::SystemdService>> = {
        let c = lock(&state.systemd_cache);
        if c.ready {
            Some(c.data.clone())
        } else {
            None
        }
    };
    if snapshot.is_none() {
        // 后台尚未完成第一次采集，直接采集一次并存入缓存
        let data = systemd::get_services()?;
        let mut c = lock(&state.systemd_cache);
        c.data = data.clone();
        c.updated_at = Instant::now();
        c.ready = true;
        snapshot = Some(data);
    }
    let mut data = snapshot.unwrap_or_default();
    systemd::sort_services(&mut data, sort_by, sort_dir);
    Ok(data)
}

/// 判断缓存是否就绪（单独读取，避免与采集写锁冲突）。
fn cache_ready(state: &Arc<AppState>, is_systemd: bool) -> bool {
    if is_systemd {
        lock(&state.systemd_cache).ready
    } else {
        lock(&state.docker_cache).ready
    }
}

pub fn invalidate_systemd_cache(state: &Arc<AppState>) {
    lock(&state.systemd_cache).ready = false;
}

pub fn refresh_systemd(state: &Arc<AppState>) {
    match systemd::get_services() {
        Ok(data) => {
            let mut c = lock(&state.systemd_cache);
            c.data = data;
            c.updated_at = Instant::now();
            c.ready = true;
        }
        Err(e) => eprintln!("[cache] systemd refresh error: {}", e),
    }
}

// ── Docker 缓存 ───────────────────────────────────────────────────

/// 对应 GetContainersFromCache：未就绪时同步采集一次。
pub fn get_containers_from_cache(
    state: &Arc<AppState>,
) -> Result<Vec<docker::Container>, String> {
    let ready = cache_ready(state, false);
    if ready {
        return Ok(lock(&state.docker_cache).data.clone());
    }
    let data = docker::get_containers()?;
    let mut c = lock(&state.docker_cache);
    c.data = data.clone();
    c.updated_at = Instant::now();
    c.ready = true;
    Ok(data)
}

pub fn invalidate_docker_cache(state: &Arc<AppState>) {
    lock(&state.docker_cache).ready = false;
}

pub fn refresh_docker(state: &Arc<AppState>) {
    match docker::get_containers() {
        Ok(data) => {
            let mut c = lock(&state.docker_cache);
            c.data = data;
            c.updated_at = Instant::now();
            c.ready = true;
        }
        Err(e) => eprintln!("[cache] docker refresh error: {}", e),
    }
}

// ── 后台刷新 ──────────────────────────────────────────────────────

/// Go 版 cacheInterval()：配置了 >0 的秒数就用它，否则默认 60s
///（注意 Go 源码注释写 30s，实现返回 60s，这里按实现保持一致）。
fn cache_interval(state: &Arc<AppState>) -> Duration {
    let secs = state
        .data
        .read()
        .unwrap_or_else(|p| p.into_inner())
        .main
        .cache_interval_sec;
    if secs > 0 {
        Duration::from_secs(secs as u64)
    } else {
        Duration::from_secs(60)
    }
}

fn feature_systemd(state: &Arc<AppState>) -> bool {
    state
        .data
        .read()
        .unwrap_or_else(|p| p.into_inner())
        .settings
        .feature_systemd
}

fn feature_docker(state: &Arc<AppState>) -> bool {
    state
        .data
        .read()
        .unwrap_or_else(|p| p.into_inner())
        .settings
        .feature_docker
}

/// 对应 StartBackgroundCache：启动两个后台线程，动态感知功能开关。
pub fn start_background_cache(state: Arc<AppState>) {
    let sd = state.clone();
    std::thread::spawn(move || {
        let interval = cache_interval(&sd);
        if feature_systemd(&sd) {
            refresh_systemd(&sd);
        }
        loop {
            std::thread::sleep(interval);
            if !feature_systemd(&sd) {
                let mut c = lock(&sd.systemd_cache);
                c.data.clear();
                c.ready = false;
                continue;
            }
            refresh_systemd(&sd);
        }
    });

    let dk = state.clone();
    std::thread::spawn(move || {
        let interval = cache_interval(&dk);
        if feature_docker(&dk) {
            refresh_docker(&dk);
        }
        loop {
            std::thread::sleep(interval);
            if !feature_docker(&dk) {
                let mut c = lock(&dk.docker_cache);
                c.data.clear();
                c.ready = false;
                continue;
            }
            refresh_docker(&dk);
        }
    });
}

/// 对应 CollectAll。
pub fn collect_all() -> MetricsSnapshot {
    system::collect_all()
}
