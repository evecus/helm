#!/usr/bin/env bash
# Helm 本地构建脚本：先构建 Vue 3 前端，产物写入 web/dist，
# 再由 rust-embed 在 cargo 编译期把前端文件内嵌进二进制。
set -e
cd "$(dirname "$0")"

echo "📦 Installing frontend dependencies..."
(cd frontend && npm install)

echo "🔨 Building Vue 3 frontend -> web/dist ..."
(cd frontend && npm run build)

echo "🦀 Compiling Rust backend (frontend embedded at compile time)..."
cargo build --release

echo ""
echo "✅ Build complete! Run: ./target/release/helm"
echo "   Default credentials: admin / admin"
