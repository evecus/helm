//! 数据持久化层：redb 单文件数据库 + AES-256-GCM 透明加密。
//!
//! 与 Go 版 internal/config/db.go 语义一致：
//!   - 数据库文件: DataDir/helm.db（redb 替代 bbolt，同为单文件嵌入式 KV）
//!   - 密钥文件:   DataDir/helm.key（首次启动自动生成，32 字节随机，0600）
//!   - 三张表对应 Go 版的三个 bucket：config / apps / settings
//!     固定 key 分别为 main / all / panel
//!   - 每个 value 以 nonce||ciphertext 形式落盘，读取时解密
//!   - 数据量小，采用全量读写：启动载入内存，保存时单事务整体覆写
//!
//! 说明：redb 文件格式与 bbolt 不通用，helm.db 与 Go 版不互通（全新安装即可）。

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;
use redb::{Database, ReadableDatabase, TableDefinition};
use std::collections::HashMap;
use std::fs::{OpenOptions, set_permissions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;

// 对应 Go 版的 bucketConfig / bucketApps / bucketSettings
const BUCKET_CONFIG: TableDefinition<&str, &[u8]> = TableDefinition::new("config");
const BUCKET_APPS: TableDefinition<&str, &[u8]> = TableDefinition::new("apps");
const BUCKET_SETTINGS: TableDefinition<&str, &[u8]> = TableDefinition::new("settings");

pub const KEY_MAIN: &str = "config/main";
pub const KEY_APPS: &str = "apps/all";
pub const KEY_PANEL: &str = "settings/panel";

const NONCE_LEN: usize = 12;

/// 读取 32 字节 AES-256 密钥；不存在则生成并落盘（0600）。
pub fn load_or_create_key(data_dir: &Path) -> Result<[u8; 32], String> {
    let path = data_dir.join("helm.key");
    if let Ok(bytes) = std::fs::read(&path) {
        if bytes.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            return Ok(key);
        }
    }
    let mut key = [0u8; 32];
    rand::rngs::OsRng
        .try_fill_bytes(&mut key)
        .map_err(|e| format!("生成加密密钥失败: {}", e))?;
    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&path)
        .map_err(|e| format!("写入加密密钥失败: {}", e))?;
    f.write_all(&key)
        .map_err(|e| format!("写入加密密钥失败: {}", e))?;
    Ok(key)
}

fn cipher(key: &[u8; 32]) -> Aes256Gcm {
    Aes256Gcm::new_from_slice(key).expect("32 字节密钥长度合法")
}

/// AES-256-GCM 加密，输出 nonce||ciphertext。
pub fn encrypt(key: &[u8; 32], plain: &[u8]) -> Result<Vec<u8>, String> {
    let mut nonce = [0u8; NONCE_LEN];
    rand::rngs::OsRng
        .try_fill_bytes(&mut nonce)
        .map_err(|e| e.to_string())?;
    let ct = cipher(key)
        .encrypt(Nonce::from_slice(&nonce), plain)
        .map_err(|e| format!("加密失败: {}", e))?;
    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

/// 解密 nonce||ciphertext 格式的数据。
pub fn decrypt(key: &[u8; 32], blob: &[u8]) -> Result<Vec<u8>, String> {
    if blob.len() < NONCE_LEN {
        return Err("密文长度非法".into());
    }
    cipher(key)
        .decrypt(Nonce::from_slice(&blob[..NONCE_LEN]), &blob[NONCE_LEN..])
        .map_err(|_| "解密失败(密钥不匹配或数据损坏)".into())
}

/// redb 数据库句柄。
pub struct Db {
    db: Database,
}

impl Db {
    /// 打开（或创建）helm.db，并确保三张表存在（对应 CreateBucketIfNotExists）。
    pub fn open(path: &Path) -> Result<Db, String> {
        let db = Database::create(path).map_err(|e| format!("打开数据库失败: {}", e))?;
        {
            let txn = db
                .begin_write()
                .map_err(|e| format!("初始化数据库失败: {}", e))?;
            {
                let _ = txn
                    .open_table(BUCKET_CONFIG)
                    .map_err(|e| format!("创建 config 表失败: {}", e))?;
                let _ = txn
                    .open_table(BUCKET_APPS)
                    .map_err(|e| format!("创建 apps 表失败: {}", e))?;
                let _ = txn
                    .open_table(BUCKET_SETTINGS)
                    .map_err(|e| format!("创建 settings 表失败: {}", e))?;
            }
            txn.commit().map_err(|e| format!("初始化数据库失败: {}", e))?;
        }
        // bbolt 以 0600 打开，这里保持一致
        let _ = set_permissions(path, std::fs::Permissions::from_mode(0o600));
        Ok(Db { db })
    }

    /// 全量读取（对应 loadAll）：返回 "config/main" / "apps/all" / "settings/panel" 的密文。
    pub fn load(&self) -> Result<HashMap<String, Vec<u8>>, String> {
        let txn = self
            .db
            .begin_read()
            .map_err(|e| format!("读取数据库失败: {}", e))?;
        let mut map = HashMap::new();

        let read_one = |table: redb::ReadOnlyTable<&str, &[u8]>, key: &str, name: &str, map: &mut HashMap<String, Vec<u8>>| -> Result<(), String> {
            if let Some(guard) = table.get(key).map_err(|e| e.to_string())? {
                map.insert(name.to_string(), guard.value().to_vec());
            }
            Ok(())
        };

        read_one(
            txn.open_table(BUCKET_CONFIG).map_err(|e| e.to_string())?,
            "main",
            KEY_MAIN,
            &mut map,
        )?;
        read_one(
            txn.open_table(BUCKET_APPS).map_err(|e| e.to_string())?,
            "all",
            KEY_APPS,
            &mut map,
        )?;
        read_one(
            txn.open_table(BUCKET_SETTINGS).map_err(|e| e.to_string())?,
            "panel",
            KEY_PANEL,
            &mut map,
        )?;
        Ok(map)
    }

    /// 全量写回（单事务，对应 persistAll）。
    pub fn save(&self, entries: &[(&str, Vec<u8>)]) -> Result<(), String> {
        let txn = self
            .db
            .begin_write()
            .map_err(|e| format!("写入数据库失败: {}", e))?;
        for (name, value) in entries {
            match *name {
                KEY_MAIN => {
                    let mut t = txn.open_table(BUCKET_CONFIG).map_err(|e| e.to_string())?;
                    t.insert("main", value.as_slice()).map_err(|e| e.to_string())?;
                }
                KEY_APPS => {
                    let mut t = txn.open_table(BUCKET_APPS).map_err(|e| e.to_string())?;
                    t.insert("all", value.as_slice()).map_err(|e| e.to_string())?;
                }
                KEY_PANEL => {
                    let mut t = txn.open_table(BUCKET_SETTINGS).map_err(|e| e.to_string())?;
                    t.insert("panel", value.as_slice()).map_err(|e| e.to_string())?;
                }
                _ => {}
            }
        }
        txn.commit().map_err(|e| format!("写入数据库失败: {}", e))?;
        Ok(())
    }
}
