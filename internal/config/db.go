package config

// 数据持久化层:bbolt 单文件数据库 + AES-256-GCM 透明加密。
//
//   - 数据库文件: DataDir/helm.db(存储 main/apps/settings 全部业务数据)
//   - 密钥文件:   DataDir/helm.key(首次启动自动生成,32 字节随机,0600 权限)
//   - 每个 value 以 nonce||ciphertext 形式落盘,读取时解密。
//   - 数据量小(几十条),采用全量读写:启动时一次载入内存,保存时整体覆写。

import (
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"time"

	"go.etcd.io/bbolt"
)

var db *bbolt.DB
var gcm cipher.AEAD

var (
	bucketConfig   = []byte("config")   // MainConfig(JWT 密钥/用户/公开模式等,不含端口)
	bucketApps     = []byte("apps")     // AppItem 列表
	bucketSettings = []byte("settings") // PanelSettings 面板配置
)

// keyPath 数据库加密密钥文件路径。
func keyPath() string { return filepath.Join(DataDir, "helm.key") }

// dbPath 数据库文件路径。
func dbPath() string { return filepath.Join(DataDir, "helm.db") }

// loadOrCreateKey 读取 32 字节 AES-256 密钥;不存在则生成并落盘(0600)。
// 注意:helm.key 丢失或被替换后,helm.db 将无法解密。
func loadOrCreateKey() ([]byte, error) {
	if key, err := os.ReadFile(keyPath()); err == nil && len(key) == 32 {
		return key, nil
	}
	key := make([]byte, 32)
	if _, err := rand.Read(key); err != nil {
		return nil, fmt.Errorf("生成加密密钥失败: %w", err)
	}
	if err := os.WriteFile(keyPath(), key, 0600); err != nil {
		return nil, fmt.Errorf("写入加密密钥失败: %w", err)
	}
	return key, nil
}

// openDB 打开数据库、初始化 AES-256-GCM 并确保 bucket 存在。
func openDB() error {
	key, err := loadOrCreateKey()
	if err != nil {
		return err
	}
	block, err := aes.NewCipher(key)
	if err != nil {
		return fmt.Errorf("初始化 AES 密钥失败: %w", err)
	}
	if gcm, err = cipher.NewGCM(block); err != nil {
		return fmt.Errorf("初始化 GCM 失败: %w", err)
	}
	if err := os.MkdirAll(DataDir, 0755); err != nil {
		return err
	}
	db, err = bbolt.Open(dbPath(), 0600, &bbolt.Options{Timeout: 5 * time.Second})
	if err != nil {
		return fmt.Errorf("打开数据库失败: %w", err)
	}
	return db.Update(func(tx *bbolt.Tx) error {
		for _, b := range [][]byte{bucketConfig, bucketApps, bucketSettings} {
			if _, err := tx.CreateBucketIfNotExists(b); err != nil {
				return err
			}
		}
		return nil
	})
}

// ── 存储层 DTO ───────────────────────────────────────────────────
// User.Password 带 json:"-"(防止 API 响应泄露哈希),因此落盘时
// 用下面不含该限制的镜像结构序列化;读取时再转回业务结构。

type storedUser struct {
	Username string `json:"username"`
	Password string `json:"password"`
	Nickname string `json:"nickname"`
	IsAdmin  bool   `json:"is_admin"`
}

type storedMain struct {
	JWTSecret        string       `json:"jwt_secret"`
	PublicMode       bool         `json:"public_mode"`
	CacheIntervalSec int          `json:"cache_interval_sec"`
	Users            []storedUser `json:"users"`
	CreatedAt        time.Time    `json:"created_at"`
}

func toStoredMain(m *MainConfig) *storedMain {
	s := &storedMain{
		JWTSecret:        m.JWTSecret,
		PublicMode:       m.PublicMode,
		CacheIntervalSec: m.CacheIntervalSec,
		Users:            make([]storedUser, 0, len(m.Users)),
		CreatedAt:        m.CreatedAt,
	}
	for _, u := range m.Users {
		s.Users = append(s.Users, storedUser{u.Username, u.Password, u.Nickname, u.IsAdmin})
	}
	return s
}

func (s *storedMain) toMain() *MainConfig {
	m := &MainConfig{
		JWTSecret:        s.JWTSecret,
		PublicMode:       s.PublicMode,
		CacheIntervalSec: s.CacheIntervalSec,
		Users:            make([]User, 0, len(s.Users)),
		CreatedAt:        s.CreatedAt,
	}
	for _, u := range s.Users {
		m.Users = append(m.Users, User{u.Username, u.Password, u.Nickname, u.IsAdmin})
	}
	return m
}

// encrypt AES-256-GCM 加密,输出 nonce||ciphertext。
func encrypt(plain []byte) []byte {
	nonce := make([]byte, gcm.NonceSize())
	rand.Read(nonce)
	return gcm.Seal(nonce, nonce, plain, nil)
}

// decrypt 解密 nonce||ciphertext 格式的数据。
func decrypt(blob []byte) ([]byte, error) {
	ns := gcm.NonceSize()
	if len(blob) < ns {
		return nil, fmt.Errorf("密文长度非法")
	}
	plain, err := gcm.Open(nil, blob[:ns], blob[ns:], nil)
	if err != nil {
		return nil, fmt.Errorf("解密失败(密钥不匹配或数据损坏): %w", err)
	}
	return plain, nil
}

// loadAll 从数据库读取全部数据到内存。
// 记录不存在(首次启动)时对应全局变量保持 nil,由 Init 写入默认值。
func loadAll() error {
	return db.View(func(tx *bbolt.Tx) error {
		if raw := tx.Bucket(bucketConfig).Get([]byte("main")); len(raw) > 0 {
			plain, err := decrypt(raw)
			if err != nil {
				return err
			}
			s := &storedMain{}
			if err := json.Unmarshal(plain, s); err != nil {
				return fmt.Errorf("解析主配置失败: %w", err)
			}
			Main = s.toMain()
		}
		if raw := tx.Bucket(bucketApps).Get([]byte("all")); len(raw) > 0 {
			plain, err := decrypt(raw)
			if err != nil {
				return err
			}
			Apps = []AppItem{}
			if err := json.Unmarshal(plain, &Apps); err != nil {
				return fmt.Errorf("解析应用列表失败: %w", err)
			}
		}
		if raw := tx.Bucket(bucketSettings).Get([]byte("panel")); len(raw) > 0 {
			plain, err := decrypt(raw)
			if err != nil {
				return err
			}
			Settings = &PanelSettings{}
			if err := json.Unmarshal(plain, Settings); err != nil {
				return fmt.Errorf("解析面板配置失败: %w", err)
			}
		}
		return nil
	})
}

// persistAll 将内存中的 Main/Apps/Settings 全量加密写回数据库(单事务)。
func persistAll() error {
	if Main == nil || Settings == nil || Apps == nil {
		return fmt.Errorf("数据未初始化,拒绝落盘")
	}
	marshalEnc := func(v interface{}) ([]byte, error) {
		b, err := json.Marshal(v)
		if err != nil {
			return nil, err
		}
		return encrypt(b), nil
	}
	mainB, err := marshalEnc(toStoredMain(Main))
	if err != nil {
		return err
	}
	appsB, err := marshalEnc(Apps)
	if err != nil {
		return err
	}
	setB, err := marshalEnc(Settings)
	if err != nil {
		return err
	}
	return db.Update(func(tx *bbolt.Tx) error {
		if err := tx.Bucket(bucketConfig).Put([]byte("main"), mainB); err != nil {
			return err
		}
		if err := tx.Bucket(bucketApps).Put([]byte("all"), appsB); err != nil {
			return err
		}
		return tx.Bucket(bucketSettings).Put([]byte("panel"), setB)
	})
}

// CloseDB 供进程退出前关闭数据库(可选调用,bbolt 会在进程退出时释放)。
func CloseDB() {
	if db != nil {
		db.Close()
	}
}
