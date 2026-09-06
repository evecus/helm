package config

import (
	"crypto/rand"
	"encoding/hex"
	"os"
	"time"

	"golang.org/x/crypto/bcrypt"
)

// 说明:全部业务数据(主配置/apps/settings)已统一持久化到 bbolt 数据库
// (helm.db,内容 AES-256-GCM 加密,见 db.go)。
// 端口不在数据库中:由启动参数 --port 指定,缺省 3088。
// 仅 uploads/ 图标文件继续存于文件系统。

// ── 主配置:密钥、用户、公开模式(不含端口) ──────────────────────

type User struct {
	Username string `json:"username"`
	Password string `json:"-"`
	Nickname string `json:"nickname"`
	IsAdmin  bool   `json:"is_admin"`
}

type MainConfig struct {
	JWTSecret        string    `json:"jwt_secret"`
	PublicMode       bool      `json:"public_mode"`
	CacheIntervalSec int       `json:"cache_interval_sec"` // 后台缓存刷新间隔（秒），默认 30
	Users            []User    `json:"users"`
	CreatedAt        time.Time `json:"created_at"`
}

// ── apps(导航条目) ─────────────────────────────────────────────

type AppItem struct {
	ID        string `json:"id"`
	Title     string `json:"title"`
	URL       string `json:"url"`      // 保留兼容旧数据
	UrlLan    string `json:"url_lan"`  // 内网地址
	UrlWan    string `json:"url_wan"`  // 公网地址
	IconType  string `json:"icon_type"`
	IconText  string `json:"icon_text"`
	IconImage string `json:"icon_image"`
	OpenType  string `json:"open_type"`
	Order     int    `json:"order"`
}

// ── settings(面板配置) ──────────────────────────────────────────

type ClockDisplay struct {
	ShowTime    bool `json:"show_time"`
	ShowDate    bool `json:"show_date"`
	ShowWeekday bool `json:"show_weekday"`
	ShowLunar   bool `json:"show_lunar"`
	ShowSeconds bool `json:"show_seconds"`
	ShowYear    bool `json:"show_year"`
}

// DisplayConfig 保存一套显示样式（桌面端或移动端）
type DisplayConfig struct {
	HostnameSize int    `json:"hostname_size"`
	ClockSize    int    `json:"clock_size"`
	IconSize     int    `json:"icon_size"`
	AppNameSize  int    `json:"app_name_size"`
	IconRadius   int    `json:"icon_radius"`
	IconGap      int    `json:"icon_gap"`
	SidePadding  int    `json:"side_padding"`
	FontHostname string `json:"font_hostname"`
	FontClock    string `json:"font_clock"`
	FontAppname  string `json:"font_appname"`
	FontUI       string `json:"font_ui"`
}

type PanelSettings struct {
	Hostname     string       `json:"hostname"`
	Logo         string       `json:"logo"`
	Wallpaper    string       `json:"wallpaper"`
	Clock        ClockDisplay `json:"clock"`
	Theme        string       `json:"theme"`
	Language     string       `json:"language"`
	// 旧版顶层字段保留（兼容未迁移数据），新版以 Desktop/Mobile 为准
	HostnameSize int          `json:"hostname_size"`
	ClockSize    int          `json:"clock_size"`
	IconSize     int          `json:"icon_size"`
	AppNameSize  int          `json:"app_name_size"`
	IconRadius   int          `json:"icon_radius"`
	IconGap      int          `json:"icon_gap"`
	SidePadding  int          `json:"side_padding"`
	FontHostname string       `json:"font_hostname"`
	FontClock    string       `json:"font_clock"`
	FontAppname  string       `json:"font_appname"`
	FontUI       string       `json:"font_ui"`
	// 桌面端 / 移动端独立样式（0 表示未设置，回退到顶层旧字段）
	Desktop      *DisplayConfig `json:"desktop,omitempty"`
	Mobile       *DisplayConfig `json:"mobile,omitempty"`
	NetworkMode  string       `json:"network_mode"` // "lan" or "wan"
	ShowAppName     bool `json:"show_app_name"`
	FeatureSysInfo  bool `json:"feature_sysinfo"`
	FeatureProcess  bool `json:"feature_process"`
	FeatureSystemd  bool `json:"feature_systemd"`
	FeatureDocker   bool `json:"feature_docker"`
}

// ── 全局状态 ─────────────────────────────────────────────────────

var (
	Main     *MainConfig
	Apps     []AppItem
	Settings *PanelSettings
	DataDir  = "data"
)

// SetDataDir 在 Init 之前调用，覆盖默认数据目录
func SetDataDir(dir string) {
	DataDir = dir
}

// ── Init ─────────────────────────────────────────────────────────

func Init() error {
	os.MkdirAll(DataDir, 0755)
	os.MkdirAll(DataDir+"/uploads", 0755)
	if err := openDB(); err != nil {
		return err
	}
	if err := loadAll(); err != nil {
		return err
	}
	if Main == nil {
		if err := createDefaultMain(); err != nil {
			return err
		}
	}
	if Apps == nil {
		Apps = []AppItem{}
	}
	if err := loadSettings(); err != nil {
		return err
	}
	return persistAll()
}

// ── 主配置默认值 ────────────────────────────────────────────────

func createDefaultMain() error {
	secret := make([]byte, 32)
	rand.Read(secret)
	hash, _ := bcrypt.GenerateFromPassword([]byte("admin"), bcrypt.DefaultCost)
	Main = &MainConfig{
		JWTSecret:  hex.EncodeToString(secret),
		PublicMode: false,
		Users:      []User{{Username: "admin", Password: string(hash), Nickname: "Admin", IsAdmin: true}},
		CreatedAt:  time.Now(),
	}
	return nil
}

// ── 持久化入口(全量写回 bbolt) ─────────────────────────────────

func SaveMain() error     { return persistAll() }
func SaveApps() error     { return persistAll() }
func SaveSettings() error { return persistAll() }

// ── settings 默认值 ──────────────────────────────────────────────

func loadSettings() error {
	if Settings == nil {
		Settings = &PanelSettings{
			Hostname:     "Helm",
			Wallpaper:    "https://images.unsplash.com/photo-1579546929518-9e396f3cc809?w=1920&q=80",
			Theme:        "purple-pink",
			Language:     "zh",
			HostnameSize: 72,
			ClockSize:    24,
			IconSize:     64,
			AppNameSize:  14,
			IconRadius:   25,
			IconGap:      22,
			SidePadding:  49,
			FontHostname: "system",
			FontClock:    "system",
			FontAppname:  "system",
			FontUI:       "system",
			ShowAppName:  true,
			Desktop: &DisplayConfig{
				HostnameSize: 72, ClockSize: 24, IconSize: 64, AppNameSize: 14,
				IconRadius: 25, IconGap: 22, SidePadding: 49,
				FontHostname: "system", FontClock: "system", FontAppname: "system", FontUI: "system",
			},
			Mobile: &DisplayConfig{
				HostnameSize: 47, ClockSize: 17, IconSize: 53, AppNameSize: 11,
				IconRadius: 25, IconGap: 13, SidePadding: 17,
				FontHostname: "system", FontClock: "system", FontAppname: "system", FontUI: "system",
			},
			FeatureSysInfo: true,
			Clock: ClockDisplay{
				ShowTime: true, ShowDate: true, ShowWeekday: true,
				ShowLunar: false, ShowSeconds: false,
			},
		}
	}
	return nil
}

// ── User helpers ──────────────────────────────────────────────────

func FindUser(username string) *User {
	for i := range Main.Users {
		if Main.Users[i].Username == username {
			return &Main.Users[i]
		}
	}
	return nil
}

// ── App helpers ───────────────────────────────────────────────────

func AddApp(app AppItem) {
	app.Order = len(Apps)
	Apps = append(Apps, app)
}

func UpdateApp(id string, updated AppItem) bool {
	for i := range Apps {
		if Apps[i].ID == id {
			updated.ID = id
			updated.Order = Apps[i].Order
			Apps[i] = updated
			return true
		}
	}
	return false
}

func DeleteApp(id string) bool {
	for i, a := range Apps {
		if a.ID == id {
			Apps = append(Apps[:i], Apps[i+1:]...)
			return true
		}
	}
	return false
}

func ReorderApps(ids []string) {
	appMap := make(map[string]AppItem)
	for _, a := range Apps {
		appMap[a.ID] = a
	}
	ordered := make([]AppItem, 0, len(Apps))
	for i, id := range ids {
		if a, ok := appMap[id]; ok {
			a.Order = i
			ordered = append(ordered, a)
		}
	}
	Apps = ordered
}
