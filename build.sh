#!/bin/bash
# Helm Build Script
# Builds the Vue 3 frontend and compiles the Go binary

set -e

echo "📦 Installing frontend dependencies..."
cd frontend
npm install

echo "🔨 Building Vue 3 frontend..."
npm run build

echo "🐹 Compiling Go backend..."
cd ..
go build -o helm .

echo ""
echo "✅ Build complete! Run: ./helm"
echo "   Default credentials: admin / admin"
