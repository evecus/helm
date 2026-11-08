//! 业务数据结构、默认值与持久化入口。
//! 与 Go 版 internal/config/config.go 一一对应。

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::db;

// ── 主配置 ────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct User {
    pub username: String,
    /// Go 版 `json:"-"`：密码不出现在任何 API 响应中；落盘用 StoredUser。
    #[serde(skip)]
    pub password: String,
    pub nickname: String,
    pub is_admin: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StoredUser {
    pub username: String,
    pub password: String,
    pub nickname: String,
    pub is_admin: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MainConfig {
    pub jwt_secret: String,
    pub public_mode: bool,
    /// 后台缓存刷新间隔（秒），0 表示使用默认 60s（与 Go 实现一致）
    pub cache_interval_sec: i32,
    pub users: Vec<User>,
    /// Go 版为 time.Time，JSON 表现为 RFC3339 字符串
    pub created_at: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StoredMain {
    pub jwt_secret: String,
    pub public_mode: bool,
    pub cache_interval_sec: i32,
    pub users: Vec<StoredUser>,
    pub created_at: String,
}

impl StoredMain {
    pub fn from_main(m: &MainConfig) -> Self {
        StoredMain {
            jwt_secret: m.jwt_secret.clone(),
            public_mode: m.public_mode,
            cache_interval_sec: m.cache_interval_sec,
            users: m
                .users
                .iter()
                .map(|u| StoredUser {
                    username: u.username.clone(),
                    password: u.password.clone(),
                    nickname: u.nickname.clone(),
                    is_admin: u.is_admin,
                })
                .collect(),
            created_at: m.created_at.clone(),
        }
    }

    pub fn into_main(self) -> MainConfig {
        MainConfig {
            jwt_secret: self.jwt_secret,
            public_mode: self.public_mode,
            cache_interval_sec: self.cache_interval_sec,
            users: self
                .users
                .into_iter()
                .map(|u| User {
                    username: u.username,
                    password: u.password,
                    nickname: u.nickname,
                    is_admin: u.is_admin,
                })
                .collect(),
            created_at: self.created_at,
        }
    }
}

// ── apps（导航条目） ──────────────────────────────────────────────

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppItem {
    pub id: String,
    pub title: String,
    pub url: String,
    pub url_lan: String,
    pub url_wan: String,
    pub icon_type: String,
    pub icon_text: String,
    pub icon_image: String,
    pub open_type: String,
    pub order: i32,
}

// ── settings（面板配置） ──────────────────────────────────────────

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ClockDisplay {
    pub show_time: bool,
    pub show_date: bool,
    pub show_weekday: bool,
    pub show_lunar: bool,
    pub show_seconds: bool,
    pub show_year: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    pub hostname_size: i32,
    pub clock_size: i32,
    pub icon_size: i32,
    pub app_name_size: i32,
    pub icon_radius: i32,
    pub icon_gap: i32,
    pub side_padding: i32,
    pub font_hostname: String,
    pub font_clock: String,
    pub font_appname: String,
    pub font_ui: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelSettings {
    pub hostname: String,
    pub logo: String,
    pub wallpaper: String,
    pub clock: ClockDisplay,
    pub theme: String,
    pub language: String,

    // 旧版顶层字段（兼容未迁移数据）
    pub hostname_size: i32,
    pub clock_size: i32,
    pub icon_size: i32,
    pub app_name_size: i32,
    pub icon_radius: i32,
    pub icon_gap: i32,
    pub side_padding: i32,
    pub font_hostname: String,
    pub font_clock: String,
    pub font_appname: String,
    pub font_ui: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub desktop: Option<DisplayConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mobile: Option<DisplayConfig>,

    pub network_mode: String,
    pub show_app_name: bool,
    pub feature_sysinfo: bool,
    pub feature_process: bool,
    pub feature_systemd: bool,
    pub feature_docker: bool,
}

/// Go 版 loadSettings() 的默认内容（逐字段对齐）。
pub fn default_settings() -> PanelSettings {
    let fonts = |hs: i32, cs: i32, is: i32, ans: i32, ir: i32, ig: i32, sp: i32| DisplayConfig {
        hostname_size: hs,
        clock_size: cs,
        icon_size: is,
        app_name_size: ans,
        icon_radius: ir,
        icon_gap: ig,
        side_padding: sp,
        font_hostname: "system".into(),
        font_clock: "system".into(),
        font_appname: "system".into(),
        font_ui: "system".into(),
    };

    PanelSettings {
        hostname: "Helm".into(),
        wallpaper: "https://images.unsplash.com/photo-1579546929518-9e396f3cc809?w=1920&q=80".into(),
        theme: "purple-pink".into(),
        language: "zh".into(),
        hostname_size: 72,
        clock_size: 24,
        icon_size: 64,
        app_name_size: 14,
        icon_radius: 25,
        icon_gap: 22,
        side_padding: 49,
        font_hostname: "system".into(),
        font_clock: "system".into(),
        font_appname: "system".into(),
        font_ui: "system".into(),
        show_app_name: true,
        desktop: Some(fonts(72, 24, 64, 14, 25, 22, 49)),
        mobile: Some(fonts(47, 17, 53, 11, 25, 13, 17)),
        feature_sysinfo: true,
        clock: ClockDisplay {
            show_time: true,
            show_date: true,
            show_weekday: true,
            show_lunar: false,
            show_seconds: false,
            show_year: false,
        },
        ..Default::default()
    }
}

/// Go 版 createDefaultMain()：32 字节随机密钥 + admin/admin 初始账号。
pub fn default_main() -> Result<MainConfig, String> {
    use rand::RngCore;
    let mut secret = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut secret);
    let hash = crate::auth::bcrypt_hash("admin")?;
    let created_at = crate::auth::now_rfc3339();
    Ok(MainConfig {
        jwt_secret: hex::encode(secret),
        public_mode: false,
        cache_interval_sec: 0,
        users: vec![User {
            username: "admin".into(),
            password: hash,
            nickname: "Admin".into(),
            is_admin: true,
        }],
        created_at,
    })
}

// ── 内存数据（对应 Go 的 Main / Apps / Settings 全局变量） ─────────

#[derive(Clone, Debug)]
pub struct Data {
    pub main: MainConfig,
    pub apps: Vec<AppItem>,
    pub settings: PanelSettings,
}

impl Data {
    /// 对应 Go config.Init()：打开数据库 → 载入 → 补默认值 → 全量写回。
    pub fn init(db: &db::Db, data_dir: &Path, key: &[u8; 32]) -> Result<Data, String> {
        std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(data_dir.join("uploads")).map_err(|e| e.to_string())?;

        let raw = db.load()?;

        let main = match raw.get("config/main") {
            Some(v) => {
                let plain = db::decrypt(key, v)?;
                let s: StoredMain = serde_json::from_slice(&plain)
                    .map_err(|e| format!("解析主配置失败: {}", e))?;
                s.into_main()
            }
            None => default_main()?,
        };

        let apps: Vec<AppItem> = match raw.get("apps/all") {
            Some(v) => {
                let plain = db::decrypt(key, v)?;
                serde_json::from_slice(&plain).map_err(|e| format!("解析应用列表失败: {}", e))?
            }
            None => Vec::new(),
        };

        let settings = match raw.get("settings/panel") {
            Some(v) => {
                let plain = db::decrypt(key, v)?;
                serde_json::from_slice(&plain).map_err(|e| format!("解析面板配置失败: {}", e))?
            }
            None => default_settings(),
        };

        let data = Data {
            main,
            apps,
            settings,
        };
        data.persist(db, key)?;
        Ok(data)
    }

    /// 对应 Go persistAll()：Main/Apps/Settings 全量加密写回（单事务）。
    pub fn persist(&self, db: &db::Db, key: &[u8; 32]) -> Result<(), String> {
        let main_b = db::encrypt(
            key,
            &serde_json::to_vec(&StoredMain::from_main(&self.main))
                .map_err(|e| e.to_string())?,
        )?;
        let apps_b = db::encrypt(key, &serde_json::to_vec(&self.apps).map_err(|e| e.to_string())?)?;
        let set_b = db::encrypt(
            key,
            &serde_json::to_vec(&self.settings).map_err(|e| e.to_string())?,
        )?;

        db.save(&[
            (db::KEY_MAIN, main_b),
            (db::KEY_APPS, apps_b),
            (db::KEY_PANEL, set_b),
        ])
    }
}

// ── helpers（对应 config.go 中的函数） ────────────────────────────

pub fn find_user<'a>(users: &'a [User], username: &str) -> Option<usize> {
    users.iter().position(|u| u.username == username)
}

pub fn add_app(apps: &mut Vec<AppItem>, mut app: AppItem) {
    app.order = apps.len() as i32;
    apps.push(app);
}

pub fn update_app(apps: &mut Vec<AppItem>, id: &str, mut updated: AppItem) -> bool {
    if let Some(pos) = apps.iter().position(|a| a.id == id) {
        updated.id = id.to_string();
        updated.order = apps[pos].order;
        apps[pos] = updated;
        return true;
    }
    false
}

pub fn delete_app(apps: &mut Vec<AppItem>, id: &str) -> bool {
    if let Some(pos) = apps.iter().position(|a| a.id == id) {
        apps.remove(pos);
        return true;
    }
    false
}

pub fn reorder_apps(apps: &mut Vec<AppItem>, ids: &[String]) {
    let mut ordered = Vec::with_capacity(apps.len());
    for (i, id) in ids.iter().enumerate() {
        if let Some(a) = apps.iter().find(|a| &a.id == id) {
            let mut a = a.clone();
            a.order = i as i32;
            ordered.push(a);
        }
    }
    *apps = ordered;
}
