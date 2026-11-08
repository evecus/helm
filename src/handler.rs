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

// ── FETCH ICON ────────────────────────────────────────────────────

pub async fn fetch_icon(req: Request) -> Response {
    let raw = query_param(&req, "url").unwrap_or_default();
    let raw = raw.trim().to_string();
    if raw.is_empty() {
        return err(StatusCode::BAD_REQUEST, "url required");
    }
    let raw = if !raw.starts_with("http://") && !raw.starts_with("https://") {
        format!("http://{}", raw)
    } else {
        raw.clone()
    };
    // 与 Go 的 url.Parse 等价：取 scheme 与 host
    let (scheme, host) = match raw.split_once("://") {
        Some((s, rest)) => {
            let authority = rest.split('/').next().unwrap_or(rest);
            (s.to_string(), authority.to_string())
        }
        None => return err(StatusCode::BAD_REQUEST, "invalid url"),
    };
    let base = format!("{}://{}", scheme, host);

    let result = match blocking(move || fetch_icon_blocking(&raw, &scheme, &base)).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    ok(json!({ "icon": result }))
}

fn fetch_icon_blocking(raw_url: &str, scheme: &str, base: &str) -> String {
    let mut icon_url = String::new();
    if let Ok(resp) = ureq::get(raw_url).timeout(Duration::from_secs(6)).call() {
        use std::io::Read;
        let mut buf = Vec::new();
        let mut reader = resp.into_reader().take(256 * 1024);
        let _ = reader.read_to_end(&mut buf);
        let html = String::from_utf8_lossy(&buf).into_owned();

        let re1 = regex::Regex::new(
            r#"(?i)<link[^>]+rel=["'](?:shortcut icon|icon|apple-touch-icon)["'][^>]+href=["']([^"']+)["']"#,
        )
        .unwrap();
        let re2 = regex::Regex::new(
            r#"(?i)<link[^>]+href=["']([^"']+)["'][^>]+rel=["'](?:shortcut icon|icon|apple-touch-icon)["']"#,
        )
        .unwrap();
        if let Some(m) = re1.captures(&html) {
            icon_url = m.get(1).map(|x| x.as_str().to_string()).unwrap_or_default();
        } else if let Some(m) = re2.captures(&html) {
            icon_url = m.get(1).map(|x| x.as_str().to_string()).unwrap_or_default();
        }
    }

    if !icon_url.is_empty() {
        if icon_url.starts_with("//") {
            format!("{}:{}", scheme, icon_url)
        } else if icon_url.starts_with('/') {
            format!("{}{}", base, icon_url)
        } else if !icon_url.starts_with("http") {
            format!("{}/{}", base, icon_url)
        } else {
            icon_url
        }
    } else {
        format!("{}/favicon.ico", base)
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
