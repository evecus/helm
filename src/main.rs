//! Helm — 自托管导航面板 / 系统监控（Rust 版，逻辑与 Go 版一致）。

use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, post, put, delete};
use axum::Router;
use rust_embed::RustEmbed;
use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;
use tower_http::catch_panic::CatchPanicLayer;

mod auth;
mod cmd;
mod collector;
mod config;
mod db;
mod handler;
mod state;

use state::AppState;

/// 版本注入：优先编译期环境变量 HELM_VERSION（对应 Go 的 -ldflags -X main.Version），
/// 否则读取项目根目录 VERSION 文件（cross 容器内也可靠）。
pub const VERSION: &str = match option_env!("HELM_VERSION") {
    Some(v) => v,
    None => include_str!("../VERSION"),
};

// ── 编译期内嵌资源（对应 Go 的 //go:embed） ───────────────────────

#[derive(RustEmbed)]
#[folder = "web/dist/"]
struct DistFiles;

#[derive(RustEmbed)]
#[folder = "assets/"]
struct Assets;

// ── 命令行参数 ────────────────────────────────────────────────────

struct Args {
    port: u16,
    dir: Option<String>,
}

fn parse_args() -> Args {
    let list: Vec<String> = std::env::args().collect();
    let mut args = Args { port: 3088, dir: None };
    let mut i = 1;
    while i < list.len() {
        let a = &list[i];
        let (key, inline) = match a.split_once('=') {
            Some((k, v)) => (k, Some(v.to_string())),
            None => (a.as_str(), None),
        };
        let take_value = |i: &mut usize| -> Option<String> {
            if let Some(v) = inline.clone() {
                Some(v)
            } else {
                *i += 1;
                list.get(*i).cloned()
            }
        };
        match key {
            "--port" => {
                if let Some(v) = take_value(&mut i) {
                    args.port = v.parse::<u16>().unwrap_or(3088);
                }
            }
            "--dir" => {
                args.dir = take_value(&mut i);
            }
            _ => {}
        }
        i += 1;
    }
    args
}

/// 对应 main.go：chdir 到可执行文件所在目录（解析符号链接）。
fn chdir_to_exe_dir() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let real = std::fs::canonicalize(&exe_dir).unwrap_or(exe_dir.clone());
    if let Err(e) = std::env::set_current_dir(&real) {
        eprintln!("Warning: could not chdir to {}: {}", real.display(), e);
    }
    exe_dir
}

// ── 静态资源服务 ──────────────────────────────────────────────────

fn bytes_response(status: StatusCode, content_type: &str, data: Vec<u8>) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(data))
        .unwrap()
}

fn not_found() -> Response {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::empty())
        .unwrap()
}

/// GET /default-wallpaper：返回内嵌的默认壁纸（对应 assets/wallpaper.*）。
async fn default_wallpaper() -> Response {
    let entry = match Assets::iter().next() {
        Some(e) => e,
        None => return not_found(),
    };
    let name = entry.as_ref();
    let file = match Assets::get(name) {
        Some(f) => f,
        None => return not_found(),
    };
    let ext = FsPath::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let mime = mime_guess::from_ext(ext)
        .first_or_octet_stream()
        .to_string();
    let mime = if mime == "application/octet-stream" {
        "image/jpeg".to_string()
    } else {
        mime
    };
    let mut resp = bytes_response(StatusCode::OK, &mime, file.data.into_owned());
    resp.headers_mut().insert(
        header::CACHE_CONTROL,
        "public, max-age=86400".parse().unwrap(),
    );
    resp
}

/// GET /uploads/*：对应 gin 的 r.Static("/uploads", DataDir/uploads)。
async fn serve_upload(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Response {
    let rel = path.trim_start_matches('/');
    if rel.is_empty() || rel.contains("..") || rel.starts_with('/') {
        return not_found();
    }
    let full = state.uploads_dir.join(rel);
    match std::fs::read(&full) {
        Ok(data) => {
            let mime = mime_guess::from_path(&full)
                .first_or_octet_stream()
                .to_string();
            bytes_response(StatusCode::OK, &mime, data)
        }
        Err(_) => not_found(),
    }
}

/// NoRoute：/assets/ 与少量根图标走真实静态文件，其余 SPA fallback 到 index.html。
async fn frontend_fallback(req: Request) -> Response {
    let path = req.uri().path();
    let is_static = path.starts_with("/assets/")
        || path == "/favicon.ico"
        || path == "/favicon.svg"
        || path == "/vite.svg";
    if is_static {
        let key = path.trim_start_matches('/');
        return match DistFiles::get(key) {
            Some(file) => {
                let mime = mime_guess::from_path(key)
                    .first_or_octet_stream()
                    .to_string();
                bytes_response(StatusCode::OK, &mime, file.data.into_owned())
            }
            None => not_found(),
        };
    }
    match DistFiles::get("index.html") {
        Some(file) => bytes_response(
            StatusCode::OK,
            "text/html; charset=utf-8",
            file.data.into_owned(),
        ),
        None => not_found(),
    }
}

// ── 中间件 ────────────────────────────────────────────────────────

/// 只打印 4xx/5xx 错误，过滤正常访问日志（对应 main.go 的自定义 Logger）。
async fn logger(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let resp = next.run(req).await;
    if resp.status().as_u16() >= 400 {
        eprintln!("[{}] {} {}", resp.status().as_u16(), method, path);
    }
    resp
}

/// 对应 main.go 的 authMiddleware：Cookie 优先，再 Bearer；校验 HS256 JWT 与 exp。
async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Response {
    let token = match auth::extract_token(&req) {
        Some(t) => t,
        None => {
            return Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
                .body(Body::from(r#"{"error":"unauthorized"}"#))
                .unwrap();
        }
    };
    let name = {
        let secret = state.read().main.jwt_secret.clone();
        auth::jwt_verify(secret.as_bytes(), &token)
    };
    match name {
        Some(n) => {
            req.extensions_mut().insert(handler::Username(n));
            next.run(req).await
        }
        None => Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
            .body(Body::from(r#"{"error":"invalid token"}"#))
            .unwrap(),
    }
}

// ── main ──────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let args = parse_args();
    let exe_dir = chdir_to_exe_dir();

    // --dir 支持绝对路径和相对路径（相对于可执行文件目录）
    let data_dir = match &args.dir {
        Some(d) => {
            let p = PathBuf::from(d);
            if p.is_absolute() {
                p
            } else {
                exe_dir.join(p)
            }
        }
        None => PathBuf::from("data"),
    };

    let state = match AppState::init(data_dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to init config: {}", e);
            std::process::exit(1);
        }
    };

    collector::start_background_cache(state.clone());

    println!("\n🚀 Helm running on http://0.0.0.0:{}\n", args.port);

    // ── Public API ────────────────────────────────────────────────
    let public: Router<Arc<AppState>> = Router::new()
        .route("/api/login", post(handler::login))
        .route("/api/panel", get(handler::get_panel_info))
        .route("/api/apps", get(handler::get_apps))
        .route("/api/checkauth", get(handler::check_auth))
        .route("/api/fetch-icon", get(handler::fetch_icon))
        .route("/api/version", get(handler::get_version));

    // ── Protected API ─────────────────────────────────────────────
    let protected: Router<Arc<AppState>> = Router::new()
        .route("/logout", post(handler::logout))
        .route("/me", get(handler::get_me))
        .route("/me/nickname", put(handler::update_nickname))
        .route("/me/password", put(handler::update_password))
        .route("/apps", post(handler::create_app))
        .route("/apps/:id", put(handler::update_app).delete(handler::delete_app))
        .route("/apps/reorder", post(handler::reorder_apps))
        .route("/upload", post(handler::upload_image))
        .route("/upload/wallpaper", post(handler::upload_wallpaper))
        .route("/upload/logo", post(handler::upload_logo))
        .route("/settings", get(handler::get_settings).put(handler::update_settings))
        .route("/publicmode", get(handler::get_public_mode).put(handler::set_public_mode))
        .route("/users", get(handler::list_users).post(handler::create_user))
        .route("/users/:username", delete(handler::delete_user))
        // System Monitor
        .route("/monitor/metrics", get(handler::get_metrics_all))
        .route("/monitor/processes", get(handler::get_processes))
        .route("/monitor/processes/:pid", delete(handler::kill_process))
        .route("/monitor/containers", get(handler::get_containers))
        .route("/monitor/containers/:id/:action", post(handler::container_post_action))
        .route("/monitor/containers/:id/logs", get(handler::get_container_logs))
        .route("/monitor/containers/:id/inspect", get(handler::inspect_container))
        .route("/monitor/compose/file", get(handler::get_compose_file))
        .route("/monitor/compose/apply", post(handler::apply_compose))
        .route("/monitor/services", get(handler::get_services))
        .route("/monitor/services/:unit/:action", post(handler::service_action))
        .route("/monitor/services/:unit/logs", get(handler::get_service_logs))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    let app: Router<Arc<AppState>> = Router::new()
        .merge(public)
        .nest("/api", protected)
        .route("/default-wallpaper", get(default_wallpaper))
        .route("/uploads/*path", get(serve_upload).head(serve_upload))
        .fallback(frontend_fallback)
        .layer(middleware::from_fn(logger))
        .layer(CatchPanicLayer::new());

    let app = app.with_state(state);

    let addr = format!("0.0.0.0:{}", args.port);
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
