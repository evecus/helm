package collector

import (
	"log"
	"sync"
	"time"

	"helm/internal/config"
)

// cacheInterval 读取配置中的刷新间隔，未设置或非法时默认 30s
func cacheInterval() time.Duration {
	if config.Main != nil && config.Main.CacheIntervalSec > 0 {
		return time.Duration(config.Main.CacheIntervalSec) * time.Second
	}
	return 60 * time.Second
}

// ── Systemd 缓存 ──────────────────────────────────────────────────

type systemdCache struct {
	mu        sync.RWMutex
	data      []SystemdService
	updatedAt time.Time
	ready     bool
}

var sdCache = &systemdCache{}

// GetServicesFromCache 返回缓存数据（如果缓存未就绪则直接采集一次）
func GetServicesFromCache(sortBy, sortDir string) ([]SystemdService, error) {
	sdCache.mu.RLock()
	ready := sdCache.ready
	var snapshot []SystemdService
	if ready {
		snapshot = make([]SystemdService, len(sdCache.data))
		copy(snapshot, sdCache.data)
	}
	sdCache.mu.RUnlock()

	if !ready {
		// 后台尚未完成第一次采集，直接采集一次并存入缓存
		data, err := GetServices()
		if err != nil {
			return nil, err
		}
		sdCache.mu.Lock()
		sdCache.data = data
		sdCache.updatedAt = time.Now()
		sdCache.ready = true
		sdCache.mu.Unlock()
		snapshot = make([]SystemdService, len(data))
		copy(snapshot, data)
	}

	SortServices(snapshot, sortBy, sortDir)
	return snapshot, nil
}

// InvalidateSystemdCache 让缓存立即失效（操作后调用，下次请求会重新采集）
func InvalidateSystemdCache() {
	sdCache.mu.Lock()
	sdCache.ready = false
	sdCache.mu.Unlock()
}

// startSystemdCache 启动 systemd 后台刷新 goroutine
// featureEnabled 是一个函数，每次 tick 时动态检查开关，避免持有旧的 bool 值
func startSystemdCache(featureEnabled func() bool) {
	go func() {
		// 启动时立即采集一次
		if featureEnabled() {
			refreshSystemd()
		}
		ticker := time.NewTicker(cacheInterval())
		defer ticker.Stop()
		for range ticker.C {
			if !featureEnabled() {
				// 功能被关闭：清空缓存，不采集
				sdCache.mu.Lock()
				sdCache.data = nil
				sdCache.ready = false
				sdCache.mu.Unlock()
				continue
			}
			refreshSystemd()
		}
	}()
}

func refreshSystemd() {
	data, err := GetServices()
	if err != nil {
		log.Printf("[cache] systemd refresh error: %v", err)
		return
	}
	sdCache.mu.Lock()
	sdCache.data = data
	sdCache.updatedAt = time.Now()
	sdCache.ready = true
	sdCache.mu.Unlock()
}

// ── Docker 缓存 ───────────────────────────────────────────────────

type dockerCache struct {
	mu        sync.RWMutex
	data      []Container
	updatedAt time.Time
	ready     bool
}

var dkCache = &dockerCache{}

// GetContainersFromCache 返回缓存数据（如果缓存未就绪则直接采集一次）
func GetContainersFromCache() ([]Container, error) {
	dkCache.mu.RLock()
	ready := dkCache.ready
	var snapshot []Container
	if ready {
		snapshot = make([]Container, len(dkCache.data))
		copy(snapshot, dkCache.data)
	}
	dkCache.mu.RUnlock()

	if !ready {
		data, err := GetContainers()
		if err != nil {
			return nil, err
		}
		dkCache.mu.Lock()
		dkCache.data = data
		dkCache.updatedAt = time.Now()
		dkCache.ready = true
		dkCache.mu.Unlock()
		snapshot = make([]Container, len(data))
		copy(snapshot, data)
	}

	return snapshot, nil
}

// InvalidateDockerCache 让缓存立即失效
func InvalidateDockerCache() {
	dkCache.mu.Lock()
	dkCache.ready = false
	dkCache.mu.Unlock()
}

// startDockerCache 启动 docker 后台刷新 goroutine
func startDockerCache(featureEnabled func() bool) {
	go func() {
		if featureEnabled() {
			refreshDocker()
		}
		ticker := time.NewTicker(cacheInterval())
		defer ticker.Stop()
		for range ticker.C {
			if !featureEnabled() {
				dkCache.mu.Lock()
				dkCache.data = nil
				dkCache.ready = false
				dkCache.mu.Unlock()
				continue
			}
			refreshDocker()
		}
	}()
}

func refreshDocker() {
	data, err := GetContainers()
	if err != nil {
		log.Printf("[cache] docker refresh error: %v", err)
		return
	}
	dkCache.mu.Lock()
	dkCache.data = data
	dkCache.updatedAt = time.Now()
	dkCache.ready = true
	dkCache.mu.Unlock()
}

// ── 统一启动入口（在 main.go 中调用）────────────────────────────

// StartBackgroundCache 启动所有后台缓存 goroutine
// featureSystemd / featureDocker 是动态读取配置的函数，
// 确保每次 tick 时都能感知最新的开关状态
func StartBackgroundCache(featureSystemd func() bool, featureDocker func() bool) {
	startSystemdCache(featureSystemd)
	startDockerCache(featureDocker)
}
