package main

import (
	"embed"
	"flag"
	"fmt"
	"io/fs"
	"log"
	"mime"
	"net/http"
	"os"
	"path/filepath"
	"strings"

	"github.com/gin-gonic/gin"
	"github.com/golang-jwt/jwt/v5"
	"helm/internal/collector"
	"helm/internal/config"
	"helm/internal/handler"
)

// Version is injected at build time via -ldflags "-X main.Version=vX.X.X"
var Version = "dev"

//go:embed web/dist
var distFS embed.FS

//go:embed assets/wallpaper.*
var assetsFS embed.FS

func main() {
	// ── 命令行参数 ────────────────────────────────────────────────
	flagPort := flag.Int("port", 0, "HTTP listening port (overrides config file)")
	flagDir := flag.String("dir", "", "Data directory path (default: ./data next to executable)")
	flag.Parse()

	if exe, err := os.Executable(); err == nil {
		exeDir := filepath.Dir(exe)
		if real, err2 := filepath.EvalSymlinks(exeDir); err2 == nil {
			exeDir = real
		}
		if err3 := os.Chdir(exeDir); err3 != nil {
			log.Printf("Warning: could not chdir to %s: %v", exeDir, err3)
		}
	}

	// ── 设置数据目录 ──────────────────────────────────────────────
	// --dir 支持绝对路径和相对路径（相对于可执行文件目录）
	if *flagDir != "" {
		dir := *flagDir
		if !filepath.IsAbs(dir) {
			if exe, err := os.Executable(); err == nil {
				dir = filepath.Join(filepath.Dir(exe), dir)
			}
		}
		config.SetDataDir(dir)
	}

	gin.SetMode(gin.ReleaseMode)

	if err := config.Init(); err != nil {
		log.Fatalf("Failed to init config: %v", err)
	}
	defer config.Close()

	// --port 优先级高于配置文件
	if *flagPort != 0 {
		config.Main.Port = *flagPort
	}

	handler.AppVersion = Version

	// 启动后台缓存（动态读取功能开关，关闭时自动停止采集）
	collector.StartBackgroundCache(
		func() bool {
			return config.Settings != nil && config.Settings.FeatureSystemd
		},
		func() bool {
			return config.Settings != nil && config.Settings.FeatureDocker
		},
	)

	fmt.Printf("\n🚀 Helm running on http://0.0.0.0:%d\n", config.Main.Port)

	// 自定义 Logger：只打印 4xx/5xx 错误，过滤正常访问日志
	r := gin.New()
	r.Use(gin.Recovery())
	r.Use(func(c *gin.Context) {
		c.Next()
		if c.Writer.Status() >= 400 {
			log.Printf("[%d] %s %s", c.Writer.Status(), c.Request.Method, c.Request.URL.Path)
		}
	})

	r.Static("/uploads", "./"+config.DataDir+"/uploads")

	// ── Public API ─────────────────────────────────────────────
	// ── Embedded default wallpaper ────────────────────────────────
	r.GET("/default-wallpaper", func(c *gin.Context) {
		entries, err := assetsFS.ReadDir("assets")
		if err != nil || len(entries) == 0 {
			c.Status(http.StatusNotFound)
			return
		}
		var wpEntry fs.DirEntry
		for _, e := range entries {
			if !e.IsDir() {
				wpEntry = e
				break
			}
		}
		if wpEntry == nil {
			c.Status(http.StatusNotFound)
			return
		}
		data, err := assetsFS.ReadFile("assets/" + wpEntry.Name())
		if err != nil {
			c.Status(http.StatusNotFound)
			return
		}
		ext := strings.ToLower(filepath.Ext(wpEntry.Name()))
		mimeType := mime.TypeByExtension(ext)
		if mimeType == "" {
			mimeType = "image/jpeg"
		}
		c.Header("Cache-Control", "public, max-age=86400")
		c.Data(http.StatusOK, mimeType, data)
	})

	r.POST("/api/login", handler.Login)
	r.GET("/api/panel", handler.GetPanelInfo)
	r.GET("/api/apps", handler.GetApps)
	r.GET("/api/checkauth", handler.CheckAuth)
	r.GET("/api/fetch-icon", handler.FetchIcon)
	r.GET("/api/version", handler.GetVersion)

	// ── Protected API ──────────────────────────────────────────
	auth := r.Group("/api", authMiddleware())
	{
		auth.POST("/logout", handler.Logout)
		auth.GET("/me", handler.GetMe)
		auth.PUT("/me/nickname", handler.UpdateNickname)
		auth.PUT("/me/password", handler.UpdatePassword)

		auth.POST("/apps", handler.CreateApp)
		auth.PUT("/apps/:id", handler.UpdateApp)
		auth.DELETE("/apps/:id", handler.DeleteApp)
		auth.POST("/apps/reorder", handler.ReorderApps)

		auth.POST("/upload", handler.UploadImage)
		auth.POST("/upload/wallpaper", handler.UploadWallpaper)
		auth.POST("/upload/logo", handler.UploadLogo)

		auth.GET("/settings", handler.GetSettings)
		auth.PUT("/settings", handler.UpdateSettings)
		auth.GET("/publicmode", handler.GetPublicMode)
		auth.PUT("/publicmode", handler.SetPublicMode)

		auth.GET("/users", handler.ListUsers)
		auth.POST("/users", handler.CreateUser)
		auth.DELETE("/users/:username", handler.DeleteUser)

		// ── System Monitor ─────────────────────────────────────────
		auth.GET("/monitor/metrics",          handler.GetMetricsAll)
		auth.GET("/monitor/processes",        handler.GetProcesses)
		auth.DELETE("/monitor/processes/:pid", handler.KillProcess)
		auth.GET("/monitor/containers",       handler.GetContainers)
		auth.POST("/monitor/containers/:id/:action", handler.ContainerAction)
		auth.GET("/monitor/containers/:id/logs",    handler.GetContainerLogs)
		auth.GET("/monitor/containers/:id/inspect", handler.InspectContainer)
		auth.POST("/monitor/containers/:id/update", handler.PullUpdateContainer)
		auth.GET("/monitor/compose/file",            handler.GetComposeFile)
		auth.POST("/monitor/compose/apply",          handler.ApplyCompose)
		auth.GET("/monitor/services",         handler.GetServices)
		auth.POST("/monitor/services/:unit/:action", handler.ServiceAction)
		auth.GET("/monitor/services/:unit/logs", handler.GetServiceLogs)
	}

	// ── Frontend (Vite SPA) ────────────────────────────────────
	sub, err := fs.Sub(distFS, "web/dist")
	if err != nil {
		log.Fatal("Failed to sub distFS:", err)
	}
	staticHandler := http.FileServer(http.FS(sub))

	r.NoRoute(func(c *gin.Context) {
		path := c.Request.URL.Path
		// Serve real static files (JS chunks, CSS, images, icons)
		if strings.HasPrefix(path, "/assets/") ||
			path == "/favicon.ico" ||
			path == "/favicon.svg" ||
			path == "/vite.svg" {
			staticHandler.ServeHTTP(c.Writer, c.Request)
			return
		}
		// SPA fallback
		indexFile, err := distFS.ReadFile("web/dist/index.html")
		if err != nil {
			c.Status(http.StatusNotFound)
			return
		}
		c.Data(http.StatusOK, "text/html; charset=utf-8", indexFile)
	})

	addr := fmt.Sprintf("0.0.0.0:%d", config.Main.Port)
	if err := r.Run(addr); err != nil {
		log.Fatal(err)
	}
}

func authMiddleware() gin.HandlerFunc {
	return func(c *gin.Context) {
		tokenStr := ""
		if cookie, err := c.Cookie("token"); err == nil {
			tokenStr = cookie
		}
		if tokenStr == "" {
			a := c.GetHeader("Authorization")
			if strings.HasPrefix(a, "Bearer ") {
				tokenStr = strings.TrimPrefix(a, "Bearer ")
			}
		}
		if tokenStr == "" {
			c.AbortWithStatusJSON(http.StatusUnauthorized, gin.H{"error": "unauthorized"})
			return
		}
		token, err := jwt.Parse(tokenStr, func(t *jwt.Token) (interface{}, error) {
			return []byte(config.Main.JWTSecret), nil
		})
		if err != nil || !token.Valid {
			c.AbortWithStatusJSON(http.StatusUnauthorized, gin.H{"error": "invalid token"})
			return
		}
		claims, ok := token.Claims.(jwt.MapClaims)
		if !ok {
			c.AbortWithStatusJSON(http.StatusUnauthorized, gin.H{"error": "invalid claims"})
			return
		}
		c.Set("username", claims["sub"])
		c.Next()
	}
}

var _ = os.Getenv
