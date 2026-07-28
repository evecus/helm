# rspanel


## 架构变化

| 项目 | 原版 (Go) | 新版 (Rust) |
|------|-----------|-------------|
| Web 框架 | Gin | Axum 0.6 |
| 系统信息 | gopsutil | sysinfo 0.29 |
| JWT | golang-jwt | jsonwebtoken |
| 密码哈希 | bcrypt | bcrypt |
| 配置格式 | YAML | YAML (serde_yaml) |
| 前端嵌入 | Go embed | rust-embed |
| 构建产物 | 单文件二进制 | 单文件二进制（静态链接 musl）|

## 快速构建

```bash
# 开发构建
./build.sh

# Release 构建（musl 静态链接，无系统依赖）
cd frontend && npm install && npm run build && cd ..
cargo build --release --target x86_64-unknown-linux-musl
```

## 目录结构

```
rspanel/
├── src/
│   ├── main.rs              # Axum 路由 + 嵌入前端
│   ├── config/mod.rs        # 配置读写（YAML/JSON）
│   ├── collector/           # 系统信息采集
│   │   ├── system.rs        # CPU/内存/磁盘/网络
│   │   ├── process.rs       # 进程管理
│   │   ├── docker.rs        # Docker 容器
│   │   ├── systemd.rs       # Systemd 服务
│   │   └── cache.rs         # 后台缓存（tokio）
│   └── handler/             # HTTP 处理器
│       ├── auth.rs          # 登录/JWT/用户管理
│       ├── panel.rs         # 面板信息/应用列表
│       ├── apps.rs          # CRUD 应用
│       ├── upload.rs        # 文件上传
│       ├── settings.rs      # 设置管理
│       └── monitor.rs       # 系统监控 API
├── frontend/                # Vue 3 前端（不变）
├── assets/                  # 内嵌壁纸
├── web/dist/                # 前端构建输出（被嵌入二进制）
├── Cargo.toml
├── Dockerfile
├── build.sh
└── .github/workflows/main.yml
```

## GitHub Actions

Workflow 改动：
- Go 工具链 → Rust (`dtolnay/rust-toolchain@stable`)
- 交叉编译目标：`x86_64-unknown-linux-musl` + `aarch64-unknown-linux-musl`
- 静态链接（musl），无 libc 依赖，可直接在 Alpine 镜像运行
- 版本号通过环境变量 `APP_VERSION` 注入
