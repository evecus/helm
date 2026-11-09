//! 全部 HTTP handler，对应 Go 版 internal/handler/handler.go。

use axum::body::Body;
use axum::extract::{Multipart, Path, Request, State};
use axum::http::{header, StatusCode};
use axum::response::Response;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::Path as FsPath;
use std::sync::Arc;
use std::time::Duration;

use crate::{auth, collector, config, state::AppState, VERSION};

// ── 通用响应工具 ──────────────────────────────────────────────────

fn json_response(status: StatusCode, v: Value) -> Response {
    let body = serde_json::to_vec(&v).unwrap_or_else(|_| b"{}".to_vec());
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .body(Body::from(body))
        .unwrap()
}

fn ok(v: Value) -> Response {
    json_response(StatusCode::OK, v)
}

fn ok_serialize<T: Serialize>(v: &T) -> Response {
    ok(serde_json::to_value(v).unwrap_or(Value::Null))
}

fn err(status: StatusCode, msg: &str) -> Response {
    json_response(status, json!({ "error": msg }))
}

fn ok_true() -> Response {
    ok(json!({ "ok": true }))
}

fn internal_error() -> Response {
    err(StatusCode::INTERNAL_SERVER_ERROR, "internal error")
}

/// 把阻塞型采集/命令执行放到 blocking 线程池。
async fn blocking<F, T>(f: F) -> Result<T, Response>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|_| internal_error())
}

async fn read_json<T: DeserializeOwned>(req: Request) -> Result<T, Response> {
    let bytes = axum::body::to_bytes(req.into_body(), 16 * 1024 * 1024)
        .await
        .map_err(|_| err(StatusCode::BAD_REQUEST, "invalid request"))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| err(StatusCode::BAD_REQUEST, &e.to_string()))
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                    out.push(h * 16 + l);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn query_param(req: &Request, key: &str) -> Option<String> {
    let q = req.uri().query()?;
    for pair in q.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if k == key {
            return Some(url_decode(v));
        }
    }
    None
}

fn random_hex8() -> String {
    use rand::RngCore;
    let mut b = [0u8; 8];
    rand::rngs::OsRng.fill_bytes(&mut b);
    hex::encode(b)
}

/// 认证中间件插入的用户名。
#[derive(Clone, Debug)]
pub struct Username(pub String);

fn username(req: &Request) -> String {
    req.extensions()
        .get::<Username>()
        .map(|u| u.0.clone())
        .unwrap_or_default()
}

// ── AUTH ──────────────────────────────────────────────────────────

pub async fn login(State(state): State<Arc<AppState>>, req: Request) -> Response {
    #[derive(serde::Deserialize)]
    #[serde(default)]
    struct Req {
        username: String,
        password: String,
    }
    impl Default for Req {
        fn default() -> Self {
            Req { username: String::new(), password: String::new() }
        }
    }
    let body: Req = match read_json(req).await {
        Ok(v) => v,
        Err(_) => return err(StatusCode::BAD_REQUEST, "invalid request"),
    };

    let (hash, user) = {
        let data = state.read();
        match config::find_user(&data.main.users, &body.username) {
            Some(i) => (
                data.main.users[i].password.clone(),
                (
                    data.main.users[i].username.clone(),
                    data.main.users[i].nickname.clone(),
                    data.main.users[i].is_admin,
                ),
            ),
            None => return err(StatusCode::UNAUTHORIZED, "invalid credentials"),
        }
    };
    if !auth::bcrypt_verify(&body.password, &hash) {
        return err(StatusCode::UNAUTHORIZED, "invalid credentials");
    }
    let token = auth::jwt_sign(state.read().main.jwt_secret.as_bytes(), &user.0);
    let mut resp = ok(json!({
        "token": token,
        "username": user.0,
        "nickname": user.1,
        "is_admin": user.2,
    }));
    resp.headers_mut().insert(
        header::SET_COOKIE,
        auth::login_cookie(&token).parse().unwrap(),
    );
    resp
}

pub async fn logout() -> Response {
    let mut resp = ok_true();
    resp.headers_mut().insert(
        header::SET_COOKIE,
        auth::logout_cookie().parse().unwrap(),
    );
    resp
}

// ── PANEL INFO (public) ───────────────────────────────────────────

fn is_mobile(req: &Request) -> bool {
    let ua = req
        .headers()
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();
    for kw in ["android", "iphone", "ipod", "windows phone", "mobile"] {
        if ua.contains(kw) {
            return true;
        }
    }
    false
}

/// 从 settings 中取出当前设备对应的样式，并做零值兜底（与 Go 版数值一致）。
fn resolve_display(s: &config::PanelSettings, mobile: bool) -> config::DisplayConfig {
    let d = if mobile { s.mobile.clone() } else { s.desktop.clone() };
    let d = d.unwrap_or_else(|| config::DisplayConfig {
        hostname_size: s.hostname_size,
        clock_size: s.clock_size,
        icon_size: s.icon_size,
        app_name_size: s.app_name_size,
        icon_radius: s.icon_radius,
        icon_gap: s.icon_gap,
        side_padding: s.side_padding,
        font_hostname: s.font_hostname.clone(),
        font_clock: s.font_clock.clone(),
        font_appname: s.font_appname.clone(),
        font_ui: s.font_ui.clone(),
    });
    config::DisplayConfig {
        hostname_size: if d.hostname_size == 0 { 56 } else { d.hostname_size },
        clock_size: if d.clock_size == 0 { 16 } else { d.clock_size },
        icon_size: if d.icon_size == 0 { 78 } else { d.icon_size },
        app_name_size: if d.app_name_size == 0 { 12 } else { d.app_name_size },
        icon_radius: if d.icon_radius == 0 { 26 } else { d.icon_radius },
        icon_gap: if d.icon_gap == 0 { 22 } else { d.icon_gap },
        side_padding: if d.side_padding == 0 { 52 } else { d.side_padding },
        font_hostname: if d.font_hostname.is_empty() { "system".into() } else { d.font_hostname },
        font_clock: if d.font_clock.is_empty() { "system".into() } else { d.font_clock },
        font_appname: if d.font_appname.is_empty() { "system".into() } else { d.font_appname },
        font_ui: if d.font_ui.is_empty() { "system".into() } else { d.font_ui },
    }
}

fn legacy_display(s: &config::PanelSettings) -> config::DisplayConfig {
    config::DisplayConfig {
        hostname_size: s.hostname_size,
        clock_size: s.clock_size,
        icon_size: s.icon_size,
        app_name_size: s.app_name_size,
        icon_radius: s.icon_radius,
        icon_gap: s.icon_gap,
        side_padding: s.side_padding,
        font_hostname: s.font_hostname.clone(),
        font_clock: s.font_clock.clone(),
        font_appname: s.font_appname.clone(),
        font_ui: s.font_ui.clone(),
    }
}

pub async fn get_panel_info(State(state): State<Arc<AppState>>, req: Request) -> Response {
    let mobile = is_mobile(&req);
    let data = state.read();
    let s = &data.settings;
    let d = resolve_display(s, mobile);
    let desktop = s.desktop.clone().unwrap_or_else(|| legacy_display(s));
    let mobile_disp = s.mobile.clone().unwrap_or_else(|| legacy_display(s));

    ok(json!({
        "hostname": s.hostname,
        "logo": s.logo,
        "wallpaper": s.wallpaper,
        "clock": s.clock,
        "theme": s.theme,
        "language": s.language,
        "hostname_size": d.hostname_size,
        "clock_size": d.clock_size,
        "icon_size": d.icon_size,
        "app_name_size": d.app_name_size,
        "icon_radius": d.icon_radius,
        "icon_gap": d.icon_gap,
        "side_padding": d.side_padding,
        "font_hostname": d.font_hostname,
        "font_clock": d.font_clock,
        "font_appname": d.font_appname,
        "font_ui": d.font_ui,
        "desktop": desktop,
        "mobile": mobile_disp,
        "is_mobile": mobile,
        "public_mode": data.main.public_mode,
        "network_mode": s.network_mode,
        "feature_sysinfo": s.feature_sysinfo,
        "feature_process": s.feature_process,
        "feature_systemd": s.feature_systemd,
        "feature_docker": s.feature_docker,
        "show_app_name": s.show_app_name,
    }))
}

pub async fn get_apps(State(state): State<Arc<AppState>>) -> Response {
    ok_serialize(&state.read().apps)
}

// ── APPS (auth required) ──────────────────────────────────────────

pub async fn create_app(State(state): State<Arc<AppState>>, req: Request) -> Response {
    let mut app: config::AppItem = match read_json(req).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    app.id = random_hex8();
    if app.open_type.is_empty() {
        app.open_type = "new_tab".into();
    }
    if app.icon_type.is_empty() {
        app.icon_type = "text".into();
    }
    {
        let mut data = state.write();
        config::add_app(&mut data.apps, app.clone());
    }
    if state.save().is_err() {
        return err(StatusCode::INTERNAL_SERVER_ERROR, "save failed");
    }
    ok_serialize(&app)
}

pub async fn update_app(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    req: Request,
) -> Response {
    let app: config::AppItem = match read_json(req).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    {
        let mut data = state.write();
        if !config::update_app(&mut data.apps, &id, app) {
            return err(StatusCode::NOT_FOUND, "not found");
        }
    }
    let _ = state.save();
    ok_true()
}

pub async fn delete_app(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    {
        let mut data = state.write();
        if !config::delete_app(&mut data.apps, &id) {
            return err(StatusCode::NOT_FOUND, "not found");
        }
    }
    let _ = state.save();
    ok_true()
}

pub async fn reorder_apps(State(state): State<Arc<AppState>>, req: Request) -> Response {
    #[derive(serde::Deserialize)]
    #[serde(default)]
    struct Req {
        ids: Vec<String>,
    }
    impl Default for Req {
        fn default() -> Self {
            Req { ids: Vec::new() }
        }
    }
    let body: Req = match read_json(req).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    {
        let mut data = state.write();
        config::reorder_apps(&mut data.apps, &body.ids);
    }
    let _ = state.save();
    ok_true()
}

// ── UPLOAD (auth required) ────────────────────────────────────────

async fn upload_file(
    state: &Arc<AppState>,
    mut multipart: Multipart,
    field: &str,
    prefix: &str,
) -> Response {
    loop {
        let f = match multipart.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(_) => return err(StatusCode::BAD_REQUEST, "no file"),
        };
        if f.name() != Some(field) {
            continue;
        }
        let filename = f.file_name().unwrap_or("").to_string();
        let ext = FsPath::new(&filename)
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        let data = match f.bytes().await {
            Ok(b) => b,
            Err(_) => return err(StatusCode::BAD_REQUEST, "no file"),
        };
        let name = format!("{}{}{}", prefix, random_hex8(), ext);
        let path = state.uploads_dir.join(&name);
        if std::fs::write(&path, &data).is_err() {
            return err(StatusCode::INTERNAL_SERVER_ERROR, "save failed");
        }
        return ok(json!({ "url": format!("/uploads/{}", name) }));
    }
    err(StatusCode::BAD_REQUEST, "no file")
}

pub async fn upload_image(
    State(state): State<Arc<AppState>>,
    multipart: Multipart,
) -> Response {
    upload_file(&state, multipart, "image", "").await
}

pub async fn upload_wallpaper(
    State(state): State<Arc<AppState>>,
    multipart: Multipart,
) -> Response {
    upload_file(&state, multipart, "wallpaper", "wp_").await
}

pub async fn upload_logo(
    State(state): State<Arc<AppState>>,
    multipart: Multipart,
) -> Response {
    upload_file(&state, multipart, "logo", "logo_").await
}

/// 对文件名做 percent-encoding，保证含空格/中文等字符时 URL 仍可用
/// （与 /uploads/*path 的 axum Path 自动解码对称）。
fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// 列出 DataDir/uploads 下已上传的图片，供「选择已有图片」弹窗使用。
/// 只返回图片扩展名，按修改时间倒序（最新在前）。
pub async fn list_uploads(State(state): State<Arc<AppState>>) -> Response {
    const IMAGE_EXTS: &[&str] = &[
        "png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "avif", "svg",
    ];

    let entries = match std::fs::read_dir(&state.uploads_dir) {
        Ok(e) => e,
        Err(_) => return ok(json!({ "files": [] })),
    };

    let mut files: Vec<(std::time::SystemTime, String)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();
        if !IMAGE_EXTS.contains(&ext.as_str()) {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        files.push((modified, format!("/uploads/{}", url_encode(&name))));
    }
    files.sort_by(|a, b| b.0.cmp(&a.0));
    let urls: Vec<String> = files.into_iter().map(|(_, u)| u).collect();
    ok(json!({ "files": urls }))
}

// ── SETTINGS (auth required) ──────────────────────────────────────

pub async fn get_settings(State(state): State<Arc<AppState>>) -> Response {
    ok_serialize(&state.read().settings)
}

pub async fn update_settings(State(state): State<Arc<AppState>>, req: Request) -> Response {
    let s: config::PanelSettings = match read_json(req).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    // 功能被关闭时，立即清除对应缓存
    {
        let data = state.read();
        if data.settings.feature_systemd && !s.feature_systemd {
            collector::invalidate_systemd_cache(&state);
        }
        if data.settings.feature_docker && !s.feature_docker {
            collector::invalidate_docker_cache(&state);
        }
    }
    state.write().settings = s;
    let _ = state.save();
    ok_true()
}

pub async fn get_public_mode(State(state): State<Arc<AppState>>) -> Response {
    ok(json!({ "public_mode": state.read().main.public_mode }))
}

pub async fn set_public_mode(State(state): State<Arc<AppState>>, req: Request) -> Response {
    #[derive(serde::Deserialize)]
    #[serde(default)]
    struct Req {
        public_mode: bool,
    }
    impl Default for Req {
        fn default() -> Self {
            Req { public_mode: false }
        }
    }
    let body: Req = match read_json(req).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    state.write().main.public_mode = body.public_mode;
    let _ = state.save();
    ok_true()
}

// ── ACCOUNT (auth required) ───────────────────────────────────────

pub async fn get_me(State(state): State<Arc<AppState>>, req: Request) -> Response {
    let name = username(&req);
    let data = state.read();
    match config::find_user(&data.main.users, &name) {
        Some(i) => ok(json!({
            "username": data.main.users[i].username,
            "nickname": data.main.users[i].nickname,
            "is_admin": data.main.users[i].is_admin,
        })),
        None => err(StatusCode::NOT_FOUND, "user not found"),
    }
}

pub async fn update_password(State(state): State<Arc<AppState>>, req: Request) -> Response {
    #[derive(serde::Deserialize)]
    #[serde(default)]
    struct Req {
        old_password: String,
        new_password: String,
    }
    impl Default for Req {
        fn default() -> Self {
            Req { old_password: String::new(), new_password: String::new() }
        }
    }
    let name = username(&req);
    let body: Req = match read_json(req).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let hash = {
        let data = state.read();
        match config::find_user(&data.main.users, &name) {
            Some(i) => data.main.users[i].password.clone(),
            None => return err(StatusCode::NOT_FOUND, "user not found"),
        }
    };
    if !auth::bcrypt_verify(&body.old_password, &hash) {
        return err(StatusCode::UNAUTHORIZED, "wrong password");
    }
    let new_hash = match auth::bcrypt_hash(&body.new_password) {
        Ok(h) => h,
        Err(_) => return internal_error(),
    };
    {
        let mut data = state.write();
        if let Some(i) = config::find_user(&data.main.users, &name) {
            data.main.users[i].password = new_hash;
        }
    }
    let _ = state.save();
    ok_true()
}

pub async fn update_nickname(State(state): State<Arc<AppState>>, req: Request) -> Response {
    #[derive(serde::Deserialize)]
    #[serde(default)]
    struct Req {
        nickname: String,
    }
    impl Default for Req {
        fn default() -> Self {
            Req { nickname: String::new() }
        }
    }
    let name = username(&req);
    let body: Req = match read_json(req).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    {
        let mut data = state.write();
        match config::find_user(&data.main.users, &name) {
            Some(i) => data.main.users[i].nickname = body.nickname,
            None => return err(StatusCode::NOT_FOUND, "user not found"),
        }
    }
    let _ = state.save();
    ok_true()
}

pub async fn list_users(State(state): State<Arc<AppState>>) -> Response {
    let data = state.read();
    let users: Vec<Value> = data
        .main
        .users
        .iter()
        .map(|u| {
            json!({
                "username": u.username,
                "nickname": u.nickname,
                "is_admin": u.is_admin,
            })
        })
        .collect();
    ok(Value::Array(users))
}

pub async fn create_user(State(state): State<Arc<AppState>>, req: Request) -> Response {
    #[derive(serde::Deserialize)]
    #[serde(default)]
    struct Req {
        username: String,
        password: String,
        nickname: String,
        is_admin: bool,
    }
    impl Default for Req {
        fn default() -> Self {
            Req { username: String::new(), password: String::new(), nickname: String::new(), is_admin: false }
        }
    }
    let body: Req = match read_json(req).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    if config::find_user(&state.read().main.users, &body.username).is_some() {
        return err(StatusCode::CONFLICT, "user exists");
    }
    let hash = match auth::bcrypt_hash(&body.password) {
        Ok(h) => h,
        Err(_) => return internal_error(),
    };
    state.write().main.users.push(config::User {
        username: body.username,
        password: hash,
        nickname: body.nickname,
        is_admin: body.is_admin,
    });
    let _ = state.save();
    ok_true()
}

pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    Path(target): Path<String>,
    req: Request,
) -> Response {
    let me = username(&req);
    if target == me {
        return err(StatusCode::BAD_REQUEST, "cannot delete self");
    }
    {
        let mut data = state.write();
        match config::find_user(&data.main.users, &target) {
            Some(i) => {
                data.main.users.remove(i);
            }
            None => return err(StatusCode::NOT_FOUND, "not found"),
        }
    }
    let _ = state.save();
    ok_true()
}

// ── CHECK AUTH (public) ───────────────────────────────────────────

pub async fn check_auth(State(state): State<Arc<AppState>>, req: Request) -> Response {
    // Go 版 CheckAuth 只读取 cookie，不检查 Authorization 头
    let token = match auth::cookie_token(&req) {
        Some(t) => t,
        None => return ok(json!({ "logged_in": false })),
    };
    let name = {
        let secret = state.read().main.jwt_secret.clone();
        auth::jwt_verify(secret.as_bytes(), &token)
    };
    let name = match name {
        Some(n) => n,
        None => return ok(json!({ "logged_in": false })),
    };
    let data = state.read();
    match config::find_user(&data.main.users, &name) {
        Some(i) => ok(json!({
            "logged_in": true,
            "username": data.main.users[i].username,
            "nickname": data.main.users[i].nickname,
            "is_admin": data.main.users[i].is_admin,
        })),
        None => ok(json!({ "logged_in": false })),
    }
}

// ── FETCH ICON (auth required) ────────────────────────────────────

/// 目标站点解析结果。
struct IconTarget {
    page_url: String, // 首页完整 URL（含路径与 query）
    root: String,     // scheme://authority
    scheme: String,
    dir: String,      // 相对 href 的基准目录
}

/// 把弹窗里填写的地址归一成可请求的 URL。
/// 支持显式 http(s)://、协议相对 //host、裸 host:port/path。
fn parse_icon_target(raw: &str, page_scheme: &str) -> Result<IconTarget, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("bad_url".into());
    }
    let full = if s.starts_with("//") {
        format!("{}:{}", page_scheme, s)
    } else if s.starts_with("http://") || s.starts_with("https://") {
        s.to_string()
    } else {
        // 裸地址默认 http；http -> https 升级由重定向跟随处理
        format!("http://{}", s)
    };
    let (scheme, rest) = full.split_once("://").ok_or("bad_url")?;
    if scheme != "http" && scheme != "https" {
        return Err("bad_url".into());
    }
    let authority_raw = rest.split('/').next().unwrap_or("");
    if authority_raw.is_empty() {
        return Err("bad_url".into());
    }
    // 去掉 userinfo（user:pass@host）
    let authority = authority_raw
        .rsplit('@')
        .next()
        .unwrap_or(authority_raw)
        .to_string();
    let root = format!("{}://{}", scheme, authority);
    let tail = &rest[authority_raw.len()..];
    let path_only = tail.split('?').next().unwrap_or(tail);
    let dir = match path_only.rfind('/') {
        Some(i) => format!("{}{}", root, &path_only[..=i]),
        None => format!("{}/", root),
    };
    let page_url = format!("{}{}", root, tail.split('#').next().unwrap_or(tail));
    Ok(IconTarget {
        page_url,
        root,
        scheme: scheme.to_string(),
        dir,
    })
}

/// 取 URL 所在目录（保留末尾 /），跳过 scheme 的 "//"。
fn url_dir(url: &str) -> String {
    let u = url
        .split('?')
        .next()
        .unwrap_or(url)
        .split('#')
        .next()
        .unwrap_or(url);
    let start = u.find("://").map(|i| i + 3).unwrap_or(0);
    match u[start..].rfind('/') {
        Some(rel) => u[..=start + rel].to_string(),
        None => format!("{}/", u),
    }
}

/// 把 <link href> / <base href> 归一成绝对 URL。
fn absolutize_icon_href(href: &str, t: &IconTarget, base_dir: &str) -> Option<String> {
    let h = href.trim();
    if h.is_empty() || h.starts_with("data:") || h.starts_with("javascript:") {
        return None;
    }
    if h.starts_with("http://") || h.starts_with("https://") {
        return Some(h.to_string());
    }
    if h.starts_with("//") {
        return Some(format!("{}:{}", t.scheme, h));
    }
    if h.starts_with('/') {
        return Some(format!("{}{}", t.root, h));
    }
    Some(format!("{}{}", base_dir, h))
}

/// 正则捕获里取第一个有值的分组（跳过 group 0）。
fn icon_attr(caps: &regex::Captures) -> &str {
    for i in 1..caps.len() {
        if let Some(m) = caps.get(i) {
            return m.as_str();
        }
    }
    ""
}

fn push_unique(v: &mut Vec<String>, s: String) {
    if !v.contains(&s) {
        v.push(s);
    }
}

/// 从首页 HTML 提取所有 <link rel~=icon> 的绝对地址（按出现顺序）。
fn extract_icon_links(html: &str, t: &IconTarget) -> Vec<String> {
    let link_re = regex::Regex::new(r"(?is)<link\b[^>]*>").unwrap();
    let base_re = regex::Regex::new(r"(?is)<base\b[^>]*>").unwrap();
    let rel_re =
        regex::Regex::new(r#"(?i)\brel\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))"#).unwrap();
    let href_re =
        regex::Regex::new(r#"(?i)\bhref\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))"#).unwrap();

    let base_dir = base_re
        .find(html)
        .and_then(|m| href_re.captures(m.as_str()))
        .and_then(|c| absolutize_icon_href(icon_attr(&c), t, &t.dir))
        .map(|u| url_dir(&u))
        .unwrap_or_else(|| t.dir.clone());

    let mut out = Vec::new();
    for m in link_re.find_iter(html) {
        let tag = m.as_str();
        let rel = match rel_re.captures(tag) {
            Some(c) => icon_attr(&c).to_ascii_lowercase(),
            None => continue,
        };
        let is_icon = rel.split_whitespace().any(|v| {
            matches!(
                v,
                "icon"
                    | "shortcut"
                    | "apple-touch-icon"
                    | "apple-touch-icon-precomposed"
                    | "mask-icon"
                    | "fluid-icon"
            )
        });
        if !is_icon {
            continue;
        }
        let href = match href_re.captures(tag) {
            Some(c) => icon_attr(&c).to_string(),
            None => continue,
        };
        if let Some(u) = absolutize_icon_href(&href, t, &base_dir) {
            push_unique(&mut out, u);
        }
    }
    out
}

/// 按内容魔数判断是否为真实图片，返回扩展名。
fn sniff_image(b: &[u8]) -> Option<&'static str> {
    if b.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some("png");
    }
    if b.starts_with(b"\xFF\xD8\xFF") {
        return Some("jpg");
    }
    if b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a") {
        return Some("gif");
    }
    if b.len() >= 12
        && b.get(..4) == Some(b"RIFF".as_slice())
        && b.get(8..12) == Some(b"WEBP".as_slice())
    {
        return Some("webp");
    }
    if b.starts_with(b"\x00\x00\x01\x00") {
        return Some("ico");
    }
    if b.starts_with(b"BM") {
        return Some("bmp");
    }
    if b.len() >= 12 && b.get(4..8) == Some(b"ftyp".as_slice()) {
        return Some("avif");
    }
    let head = String::from_utf8_lossy(&b[..b.len().min(1024)]).to_ascii_lowercase();
    if head.contains("<svg") {
        return Some("svg");
    }
    None
}

/// 请求候选地址，仅接受 2xx 且内容确实是图片的响应。
fn fetch_icon_bytes(agent: &ureq::Agent, url: &str) -> Option<(&'static str, Vec<u8>)> {
    use std::io::Read;
    const MAX: usize = 2 * 1024 * 1024;
    let resp = agent
        .get(url)
        .set(
            "Accept",
            "image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8",
        )
        .call()
        .ok()?;
    if !(200..300).contains(&resp.status()) {
        return None;
    }
    let ct = resp
        .header("Content-Type")
        .unwrap_or("")
        .to_ascii_lowercase();
    if ct.starts_with("text/html") {
        return None;
    }
    let mut buf = Vec::new();
    resp.into_reader()
        .take(MAX as u64 + 1)
        .read_to_end(&mut buf)
        .ok()?;
    if buf.is_empty() || buf.len() > MAX {
        return None;
    }
    Some((sniff_image(&buf)?, buf))
}

fn fetch_icon_blocking(t: &IconTarget, uploads_dir: &FsPath) -> Result<String, String> {
    use std::io::Read;

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(6))
        .redirects(5)
        .user_agent("Mozilla/5.0 (compatible; HelmIconFetcher/1.0)")
        .build();

    // 1) 抓首页 HTML，提取 <link rel~icon> 的 href（含 <base href> 基准）
    let mut candidates: Vec<String> = Vec::new();
    if let Ok(resp) = agent.get(&t.page_url).call() {
        if (200..300).contains(&resp.status()) {
            let mut buf = Vec::new();
            if resp
                .into_reader()
                .take(256 * 1024)
                .read_to_end(&mut buf)
                .is_ok()
            {
                let html = String::from_utf8_lossy(&buf).into_owned();
                candidates = extract_icon_links(&html, t);
            }
        }
    }

    // 2) 追加常见兜底路径（根路径 + 子路径，去重）
    for p in [
        "/favicon.ico",
        "/favicon.png",
        "/favicon.svg",
        "/apple-touch-icon.png",
        "/apple-touch-icon-precomposed.png",
    ] {
        push_unique(&mut candidates, format!("{}{}", t.root, p));
    }
    if t.dir != format!("{}/", t.root) {
        for p in ["favicon.ico", "favicon.png", "favicon.svg"] {
            push_unique(&mut candidates, format!("{}{}", t.dir, p));
        }
    }

    // 3) 逐个验证，取第一个真实图片落盘到 uploads（同源，避免 Mixed Content）
    for url in &candidates {
        if let Some((ext, bytes)) = fetch_icon_bytes(&agent, url) {
            let name = format!("ico_{}.{}", random_hex8(), ext);
            if std::fs::write(uploads_dir.join(&name), &bytes).is_ok() {
                return Ok(format!("/uploads/{}", url_encode(&name)));
            }
        }
    }
    Err("icon_not_found".into())
}

pub async fn fetch_icon(State(state): State<Arc<AppState>>, req: Request) -> Response {
    let raw = query_param(&req, "url").unwrap_or_default();
    let page_scheme = query_param(&req, "scheme").unwrap_or_default();
    let page_scheme = if page_scheme == "https" { "https" } else { "http" };
    let target = match parse_icon_target(&raw, page_scheme) {
        Ok(t) => t,
        Err(e) => return err(StatusCode::BAD_REQUEST, &e),
    };
    let uploads_dir = state.uploads_dir.clone();
    let result = match blocking(move || fetch_icon_blocking(&target, &uploads_dir)).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    match result {
        Ok(url) => ok(json!({ "icon": url })),
        Err(e) => err(StatusCode::BAD_GATEWAY, &e),
    }
}

// ── VERSION ───────────────────────────────────────────────────────

pub async fn get_version() -> Response {
    ok(json!({ "version": VERSION }))
}

// ── SYSTEM MONITOR APIs ───────────────────────────────────────────

pub async fn get_metrics_all() -> Response {
    match blocking(collector::collect_all).await {
        Ok(v) => ok_serialize(&v),
        Err(r) => r,
    }
}

pub async fn get_processes(req: Request) -> Response {
    let sort_by = query_param(&req, "sort").unwrap_or_else(|| "cpu".into());
    let sort_dir = query_param(&req, "dir").unwrap_or_else(|| "desc".into());
    let limit = query_param(&req, "limit")
        .unwrap_or_else(|| "100".into())
        .parse::<i32>()
        .unwrap_or(0);
    let res = match blocking(move || collector::process::get_processes(&sort_by, &sort_dir, limit)).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    match res {
        Ok(procs) => ok_serialize(&procs),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

pub async fn kill_process(Path(pid): Path<String>) -> Response {
    let pid = match pid.parse::<i32>() {
        Ok(v) => v,
        Err(_) => return err(StatusCode::BAD_REQUEST, "invalid pid"),
    };
    let res = match blocking(move || collector::process::kill_process(pid)).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    match res {
        Ok(()) => ok_true(),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

pub async fn get_containers(State(state): State<Arc<AppState>>) -> Response {
    let st = state.clone();
    let res = match blocking(move || collector::get_containers_from_cache(&st)).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    match res {
        Ok(v) => ok_serialize(&v),
        Err(_) => ok(Value::Array(Vec::new())),
    }
}

/// Go 版同时注册了 `/containers/:id/:action` 与静态的 `/containers/:id/update`，
/// gin 静态优先：update 走 PullUpdateContainer，其余按 start/stop/restart 白名单处理。
pub async fn container_post_action(
    State(state): State<Arc<AppState>>,
    Path((id, action)): Path<(String, String)>,
) -> Response {
    if action == "update" {
        let st = state.clone();
        let cid = id.clone();
        let (log, result) = match blocking(move || collector::docker::pull_and_update_container(&cid)).await {
            Ok(v) => v,
            Err(r) => return r,
        };
        return match result {
            Ok(()) => {
                collector::invalidate_docker_cache(&st);
                ok(json!({ "log": log }))
            }
            Err(e) => json_response(StatusCode::INTERNAL_SERVER_ERROR, json!({ "error": e, "log": log })),
        };
    }
    if !matches!(action.as_str(), "start" | "stop" | "restart") {
        return err(StatusCode::BAD_REQUEST, "invalid action");
    }
    let cid = id.clone();
    let act = action.clone();
    let res = match blocking(move || collector::docker::container_action(&cid, &act)).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    match res {
        Ok(()) => {
            collector::invalidate_docker_cache(&state);
            ok_true()
        }
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

pub async fn get_container_logs(Path(id): Path<String>, req: Request) -> Response {
    let lines = query_param(&req, "lines")
        .unwrap_or_else(|| "200".into())
        .parse::<i32>()
        .unwrap_or(0);
    let res = match blocking(move || collector::docker::get_container_logs(&id, lines)).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    match res {
        Ok(logs) => ok(json!({ "logs": logs })),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

pub async fn inspect_container(Path(id): Path<String>) -> Response {
    let res = match blocking(move || collector::docker::inspect_container(&id)).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    match res {
        Ok(v) => ok_serialize(&v),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

pub async fn get_compose_file(req: Request) -> Response {
    let path = query_param(&req, "path").unwrap_or_default();
    if path.is_empty() {
        return err(StatusCode::BAD_REQUEST, "path required");
    }
    let res = match blocking(move || collector::docker::read_compose_file(&path)).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    match res {
        Ok(content) => ok(json!({ "content": content })),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

pub async fn apply_compose(State(state): State<Arc<AppState>>, req: Request) -> Response {
    #[derive(serde::Deserialize)]
    #[serde(default)]
    struct Body {
        path: String,
        content: String,
        container_id: String,
    }
    impl Default for Body {
        fn default() -> Self {
            Body { path: String::new(), content: String::new(), container_id: String::new() }
        }
    }
    let body: Body = match read_json(req).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let (log, result) = match blocking(move || {
        collector::docker::write_and_apply_compose(&body.path, &body.content, &body.container_id)
    })
    .await
    {
        Ok(v) => v,
        Err(r) => return r,
    };
    match result {
        Ok(()) => {
            collector::invalidate_docker_cache(&state);
            ok(json!({ "message": "重建成功", "log": log }))
        }
        Err(e) => json_response(StatusCode::INTERNAL_SERVER_ERROR, json!({ "error": e, "log": log })),
    }
}

pub async fn get_services(State(state): State<Arc<AppState>>, req: Request) -> Response {
    let sort_by = query_param(&req, "sort").unwrap_or_default();
    let sort_dir = query_param(&req, "dir").unwrap_or_else(|| "desc".into());
    let st = state.clone();
    let res = match blocking(move || collector::get_services_from_cache(&st, &sort_by, &sort_dir)).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    match res {
        Ok(v) => ok_serialize(&v),
        Err(_) => ok(Value::Array(Vec::new())),
    }
}

pub async fn service_action(
    State(state): State<Arc<AppState>>,
    Path((unit, action)): Path<(String, String)>,
) -> Response {
    if !matches!(action.as_str(), "start" | "stop" | "restart" | "enable" | "disable") {
        return err(StatusCode::BAD_REQUEST, "invalid action");
    }
    let u = unit.clone();
    let a = action.clone();
    let res = match blocking(move || collector::systemd::service_action(&u, &a)).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    match res {
        Ok(()) => {
            collector::invalidate_systemd_cache(&state);
            ok_true()
        }
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

pub async fn get_service_logs(Path(unit): Path<String>, req: Request) -> Response {
    let lines = query_param(&req, "lines")
        .unwrap_or_else(|| "200".into())
        .parse::<i32>()
        .unwrap_or(0);
    let res = match blocking(move || collector::systemd::get_service_logs(&unit, lines)).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    match res {
        Ok(logs) => ok(json!({ "logs": logs })),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}
