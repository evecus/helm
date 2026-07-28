package config

import (
	"database/sql"
	"fmt"

	_ "modernc.org/sqlite"
)

// db 是全局 SQLite 连接。apps 和 settings 均存储于此，
// 仅 uploads/ 目录下的图标图片文件继续保存在文件系统中。
var db *sql.DB

// dbPath 返回数据库文件路径，位于 DataDir 根目录下。
func dbPath() string {
	return DataDir + "/helm.db"
}

// openDB 打开（或创建）SQLite 数据库并建表。
func openDB() error {
	conn, err := sql.Open("sqlite", dbPath())
	if err != nil {
		return fmt.Errorf("打开数据库失败: %w", err)
	}
	// SQLite 不支持多连接并发写，限制为单连接以避免 "database is locked"
	conn.SetMaxOpenConns(1)

	if _, err := conn.Exec(`PRAGMA journal_mode=WAL;`); err != nil {
		return fmt.Errorf("设置 WAL 模式失败: %w", err)
	}
	if _, err := conn.Exec(`PRAGMA foreign_keys=ON;`); err != nil {
		return fmt.Errorf("启用外键约束失败: %w", err)
	}

	if err := migrateSchema(conn); err != nil {
		return fmt.Errorf("初始化表结构失败: %w", err)
	}

	db = conn
	return nil
}

// migrateSchema 创建所需的表（若不存在）。
func migrateSchema(conn *sql.DB) error {
	stmts := []string{
		`CREATE TABLE IF NOT EXISTS apps (
			id         TEXT PRIMARY KEY,
			title      TEXT NOT NULL DEFAULT '',
			url        TEXT NOT NULL DEFAULT '',
			url_lan    TEXT NOT NULL DEFAULT '',
			url_wan    TEXT NOT NULL DEFAULT '',
			icon_type  TEXT NOT NULL DEFAULT '',
			icon_text  TEXT NOT NULL DEFAULT '',
			icon_image TEXT NOT NULL DEFAULT '',
			open_type  TEXT NOT NULL DEFAULT '',
			sort_order INTEGER NOT NULL DEFAULT 0
		);`,
		`CREATE INDEX IF NOT EXISTS idx_apps_sort_order ON apps(sort_order);`,
		// settings 以单行 KV 形式存储：固定 id=1，value 为整份 PanelSettings 的 JSON。
		// 面板设置是单例配置，读写整份即可，无需拆分为多列。
		`CREATE TABLE IF NOT EXISTS settings (
			id    INTEGER PRIMARY KEY CHECK (id = 1),
			value TEXT NOT NULL DEFAULT '{}'
		);`,
	}
	for _, s := range stmts {
		if _, err := conn.Exec(s); err != nil {
			return err
		}
	}
	return nil
}

// closeDB 关闭数据库连接，供程序退出时调用。
func closeDB() error {
	if db == nil {
		return nil
	}
	return db.Close()
}
