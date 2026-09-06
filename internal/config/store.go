package config

import (
	"bytes"
	"encoding/gob"
	"fmt"
	"os"
	"path/filepath"
)

// storeData 是 gob 持久化的完整数据结构。
// apps 与 settings 均存储于 DataDir 根目录下的单个 gob 文件中,
// 仅 uploads/ 目录下的图标图片文件继续保存在文件系统中。
type storeData struct {
	Apps     []AppItem
	Settings *PanelSettings
}

// storePath 返回 gob 数据文件路径,位于 DataDir 根目录下。
func storePath() string {
	return filepath.Join(DataDir, "store.gob")
}

// loadStore 从 gob 文件加载数据到内存。
// 文件不存在时(首次启动)不视为错误:Apps 置为空切片,
// Settings 置为 nil,由 loadSettings 负责写入默认值。
func loadStore() error {
	data, err := os.ReadFile(storePath())
	if os.IsNotExist(err) {
		Apps = []AppItem{}
		Settings = nil
		return nil
	}
	if err != nil {
		return fmt.Errorf("读取数据文件失败: %w", err)
	}
	var s storeData
	if err := gob.NewDecoder(bytes.NewReader(data)).Decode(&s); err != nil {
		return fmt.Errorf("解析数据文件失败: %w", err)
	}
	Apps = s.Apps
	Settings = s.Settings
	if Apps == nil {
		Apps = []AppItem{}
	}
	return nil
}

// persistStore 将内存中的 Apps 与 Settings 全量写入 gob 文件。
// 先写临时文件再原子重命名,避免写一半崩溃导致数据损坏。
// 数据量小(几十条),全量写入比增量更新更简单可靠,
// 且 apps 与 settings 本就同属一份面板状态,整体落盘语义最清晰。
func persistStore() error {
	var buf bytes.Buffer
	if err := gob.NewEncoder(&buf).Encode(storeData{Apps: Apps, Settings: Settings}); err != nil {
		return fmt.Errorf("序列化数据失败: %w", err)
	}
	tmp := storePath() + ".tmp"
	if err := os.WriteFile(tmp, buf.Bytes(), 0600); err != nil {
		return fmt.Errorf("写入临时文件失败: %w", err)
	}
	if err := os.Rename(tmp, storePath()); err != nil {
		os.Remove(tmp)
		return fmt.Errorf("替换数据文件失败: %w", err)
	}
	return nil
}
