//! 全局应用状态，对应 Go 版的包级全局变量（Main / Apps / Settings / caches）。

use crate::{collector, config, db};
use std::path::PathBuf;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub struct AppState {
    /// DataDir/uploads（对应 Go 版 config.DataDir + "/uploads"）
    pub uploads_dir: PathBuf,
    /// helm.key 中的 AES-256 密钥
    pub key: [u8; 32],
    /// redb 数据库句柄
    pub db: db::Db,
    /// Main / Apps / Settings
    pub data: RwLock<config::Data>,
    pub systemd_cache: collector::Cache<collector::systemd::SystemdService>,
    pub docker_cache: collector::Cache<collector::docker::Container>,
}

impl AppState {
    /// 对应 main.go 的 config.Init()。
    pub fn init(data_dir: PathBuf) -> Result<Arc<AppState>, String> {
        std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
        let uploads_dir = data_dir.join("uploads");
        std::fs::create_dir_all(&uploads_dir).map_err(|e| e.to_string())?;

        let key = db::load_or_create_key(&data_dir)?;
        let db = db::Db::open(&data_dir.join("helm.db"))?;
        let data = config::Data::init(&db, &data_dir, &key)?;

        Ok(Arc::new(AppState {
            uploads_dir,
            key,
            db,
            data: RwLock::new(data),
            systemd_cache: collector::Cache::new(),
            docker_cache: collector::Cache::new(),
        }))
    }

    pub fn read(&self) -> RwLockReadGuard<'_, config::Data> {
        self.data.read().unwrap_or_else(|p| p.into_inner())
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, config::Data> {
        self.data.write().unwrap_or_else(|p| p.into_inner())
    }

    /// 对应 config.SaveMain/SaveApps/SaveSettings → persistAll。
    pub fn save(&self) -> Result<(), String> {
        self.read().persist(&self.db, &self.key)
    }
}
