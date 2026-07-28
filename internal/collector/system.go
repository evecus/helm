package collector

import (
	"fmt"
	"net"
	"os"
	"os/exec"
	"runtime"
	"strings"
	"time"

	"github.com/shirou/gopsutil/v3/cpu"
	"github.com/shirou/gopsutil/v3/disk"
	"github.com/shirou/gopsutil/v3/host"
	"github.com/shirou/gopsutil/v3/load"
	"github.com/shirou/gopsutil/v3/mem"
	psnet "github.com/shirou/gopsutil/v3/net"
)

type SystemInfo struct {
	Hostname        string `json:"hostname"`
	OS              string `json:"os"`
	Platform        string `json:"platform"`
	PlatformVersion string `json:"platform_version"`
	KernelVersion   string `json:"kernel_version"`
	Arch            string `json:"arch"`
	Uptime          uint64 `json:"uptime"`
	UptimeStr       string `json:"uptime_str"`
	BootTime        uint64 `json:"boot_time"`
	CPUModel        string `json:"cpu_model"`
	CPUCores        int32  `json:"cpu_cores"`
	CPUThreads      int    `json:"cpu_threads"`
	PublicIP        string `json:"public_ip"`
	LocalIPv4       string `json:"local_ipv4"`
}

type CPUStats struct {
	UsagePercent float64   `json:"usage_percent"`
	PerCoreUsage []float64 `json:"per_core_usage"`
	LoadAvg1     float64   `json:"load_avg_1"`
	LoadAvg5     float64   `json:"load_avg_5"`
	LoadAvg15    float64   `json:"load_avg_15"`
	FrequencyMHz float64   `json:"frequency_mhz"`
}

type MemoryStats struct {
	Total       uint64  `json:"total"`
	Used        uint64  `json:"used"`
	Free        uint64  `json:"free"`
	Available   uint64  `json:"available"`
	UsedPercent float64 `json:"used_percent"`
	SwapTotal   uint64  `json:"swap_total"`
	SwapUsed    uint64  `json:"swap_used"`
	SwapFree    uint64  `json:"swap_free"`
	SwapPercent float64 `json:"swap_percent"`
	Cached      uint64  `json:"cached"`
	Buffers     uint64  `json:"buffers"`
}

type DiskPartition struct {
	Device      string  `json:"device"`
	Mountpoint  string  `json:"mountpoint"`
	Fstype      string  `json:"fstype"`
	Total       uint64  `json:"total"`
	Used        uint64  `json:"used"`
	Free        uint64  `json:"free"`
	UsedPercent float64 `json:"used_percent"`
	ReadBytes   uint64  `json:"read_bytes"`
	WriteBytes  uint64  `json:"write_bytes"`
}

type DiskStats struct {
	Partitions []DiskPartition `json:"partitions"`
}

type NetworkInterface struct {
	Name        string   `json:"name"`
	BytesSent   uint64   `json:"bytes_sent"`
	BytesRecv   uint64   `json:"bytes_recv"`
	PacketsSent uint64   `json:"packets_sent"`
	PacketsRecv uint64   `json:"packets_recv"`
	SpeedUp     uint64   `json:"speed_up"`
	SpeedDown   uint64   `json:"speed_down"`
	Addrs       []string `json:"addrs"`
}

type NetworkStats struct {
	Interfaces   []NetworkInterface `json:"interfaces"`
	TotalSent    uint64             `json:"total_sent"`
	TotalRecv    uint64             `json:"total_recv"`
	Connections  int                `json:"connections"`
}

type Temperature struct {
	Sensor string  `json:"sensor"`
	Temp   float64 `json:"temperature"`
}

type MetricsSnapshot struct {
	Timestamp int64        `json:"timestamp"`
	System    SystemInfo   `json:"system"`
	CPU       CPUStats     `json:"cpu"`
	Memory    MemoryStats  `json:"memory"`
	Disk      DiskStats    `json:"disk"`
	Network   NetworkStats `json:"network"`
	Temps     []Temperature `json:"temperatures"`
}

// track previous network counters for speed calculation
var prevNetStats = map[string]psnet.IOCountersStat{}
var prevNetTime  = time.Now()

func GetSystemInfo() SystemInfo {
	info, _ := host.Info()
	cpuInfos, _ := cpu.Info()
	threads, _ := cpu.Counts(true)   // 逻辑线程数（含超线程）
	physCores, _ := cpu.Counts(false) // 物理核心数

	si := SystemInfo{Arch: runtime.GOARCH, CPUThreads: threads}
	if info != nil {
		si.Hostname = info.Hostname
		si.OS = info.OS
		si.Platform = info.Platform
		si.PlatformVersion = info.PlatformVersion
		si.KernelVersion = info.KernelVersion
		si.Uptime = info.Uptime
		si.UptimeStr = formatUptime(info.Uptime)
		si.BootTime = info.BootTime
	}
	if len(cpuInfos) > 0 {
		si.CPUModel = cpuInfos[0].ModelName
	}
	// cpu.Counts(false) 直接统计唯一物理核心，比 cpuInfos[0].Cores 准确
	if physCores > 0 {
		si.CPUCores = int32(physCores)
	} else if len(cpuInfos) > 0 {
		// 兜底：遍历所有条目，统计唯一 "physicalId:coreId" 组合
		seen := map[string]struct{}{}
		for _, c := range cpuInfos {
			seen[c.PhysicalID+":"+c.CoreID] = struct{}{}
		}
		if len(seen) > 0 {
			si.CPUCores = int32(len(seen))
		} else {
			si.CPUCores = cpuInfos[0].Cores
		}
	}

	// Local IPv4 - 只取第一个私网 IPv4 地址
	ifaces, _ := net.Interfaces()
	for _, iface := range ifaces {
		if iface.Flags&net.FlagLoopback != 0 { continue }
		addrs, _ := iface.Addrs()
		for _, addr := range addrs {
			var ip net.IP
			switch v := addr.(type) {
			case *net.IPNet:
				ip = v.IP
			case *net.IPAddr:
				ip = v.IP
			}
			if ip == nil || ip.IsLoopback() || ip.IsLinkLocalUnicast() { continue }
			if ip.To4() == nil { continue } // 只要 IPv4
			if si.LocalIPv4 == "" {
				si.LocalIPv4 = addr.String() // 带 CIDR，如 10.0.0.2/24
			}
		}
	}
	return si
}

func GetCPUStats() CPUStats {
	usage, _ := cpu.Percent(300*time.Millisecond, false)
	perCore, _ := cpu.Percent(0, true)
	lavg, _ := load.Avg()
	cpuInfos, _ := cpu.Info()

	stats := CPUStats{PerCoreUsage: perCore}
	if len(usage) > 0 { stats.UsagePercent = usage[0] }
	if lavg != nil {
		stats.LoadAvg1 = lavg.Load1
		stats.LoadAvg5 = lavg.Load5
		stats.LoadAvg15 = lavg.Load15
	}
	if len(cpuInfos) > 0 { stats.FrequencyMHz = cpuInfos[0].Mhz }
	return stats
}

func GetMemoryStats() MemoryStats {
	v, _ := mem.VirtualMemory()
	s, _ := mem.SwapMemory()
	stats := MemoryStats{}
	if v != nil {
		stats.Total = v.Total; stats.Used = v.Used
		stats.Free = v.Free; stats.Available = v.Available
		stats.UsedPercent = v.UsedPercent
		stats.Cached = v.Cached; stats.Buffers = v.Buffers
	}
	if s != nil {
		stats.SwapTotal = s.Total; stats.SwapUsed = s.Used
		stats.SwapFree = s.Free; stats.SwapPercent = s.UsedPercent
	}
	return stats
}

func GetDiskStats() DiskStats {
	parts, _ := disk.Partitions(false)
	ios, _ := disk.IOCounters()
	var partitions []DiskPartition
	seen := map[string]bool{}
	for _, p := range parts {
		if seen[p.Mountpoint] { continue }
		if !strings.HasPrefix(p.Mountpoint, "/") { continue }
		u, err := disk.Usage(p.Mountpoint)
		if err != nil || u.Total == 0 { continue }
		seen[p.Mountpoint] = true
		dp := DiskPartition{
			Device: p.Device, Mountpoint: p.Mountpoint,
			Fstype: p.Fstype, Total: u.Total,
			Used: u.Used, Free: u.Free, UsedPercent: u.UsedPercent,
		}
		// match IO counter by device name
		devName := p.Device
		if idx := strings.LastIndex(devName, "/"); idx >= 0 {
			devName = devName[idx+1:]
		}
		if io, ok := ios[devName]; ok {
			dp.ReadBytes = io.ReadBytes
			dp.WriteBytes = io.WriteBytes
		}
		partitions = append(partitions, dp)
	}
	return DiskStats{Partitions: partitions}
}

func isRealInterface(name string) bool {
	// 1. 读 type 文件，非 Ethernet(1) 直接排除（sit=776, ip6tnl=769 等隧道）
	typeBytes, err := os.ReadFile("/sys/class/net/" + name + "/type")
	if err != nil {
		return false
	}
	if strings.TrimSpace(string(typeBytes)) != "1" {
		return false
	}

	// 2. 通过 sysfs 符号链接判断是否为物理设备
	dest, err := os.Readlink("/sys/class/net/" + name)
	if err != nil {
		return false
	}

	// 非 virtual 路径 → 物理网卡，直接保留
	if !strings.Contains(dest, "/virtual/") {
		return true
	}

	// 3. virtual 设备：有 peer_ifindex 的是 veth（容器点对点网卡），排除
	if _, err := os.Stat("/sys/class/net/" + name + "/peer_ifindex"); err == nil {
		return false
	}

	// 4. virtual 设备：有 bridge 目录的是网桥（docker0, br-xxx）
	//    仅当系统安装了 docker 时才保留
	if _, err := os.Stat("/sys/class/net/" + name + "/bridge"); err == nil {
		return dockerInstalled()
	}

	return false
}

func dockerInstalled() bool {
	_, err := exec.LookPath("docker")
	return err == nil
}

func GetNetworkStats() NetworkStats {
	ifaces, _ := psnet.Interfaces()
	ios, _ := psnet.IOCounters(true)
	now := time.Now()
	elapsed := now.Sub(prevNetTime).Seconds()
	if elapsed < 0.1 { elapsed = 1 }

	ioMap := map[string]psnet.IOCountersStat{}
	for _, io := range ios { ioMap[io.Name] = io }

	var interfaces []NetworkInterface
	var totalSent, totalRecv uint64
	for _, iface := range ifaces {
		if !isRealInterface(iface.Name) { continue }
		ni := NetworkInterface{Name: iface.Name}
		for _, addr := range iface.Addrs {
			ni.Addrs = append(ni.Addrs, addr.Addr)
		}
		if io, ok := ioMap[iface.Name]; ok {
			ni.BytesSent = io.BytesSent
			ni.BytesRecv = io.BytesRecv
			ni.PacketsSent = io.PacketsSent
			ni.PacketsRecv = io.PacketsRecv
			totalSent += io.BytesSent
			totalRecv += io.BytesRecv
			if prev, ok := prevNetStats[iface.Name]; ok {
				up := float64(io.BytesSent-prev.BytesSent) / elapsed
				dn := float64(io.BytesRecv-prev.BytesRecv) / elapsed
				if up > 0 { ni.SpeedUp = uint64(up) }
				if dn > 0 { ni.SpeedDown = uint64(dn) }
			}
		}
		interfaces = append(interfaces, ni)
	}

	// Save for next call
	prevNetStats = ioMap
	prevNetTime = now

	conns, _ := psnet.Connections("all")

	return NetworkStats{
		Interfaces:  interfaces,
		TotalSent:   totalSent,
		TotalRecv:   totalRecv,
		Connections: len(conns),
	}
}

func GetTemperatures() []Temperature {
	var temps []Temperature
	for i := 0; i < 10; i++ {
		data, err := os.ReadFile(fmt.Sprintf("/sys/class/thermal/thermal_zone%d/temp", i))
		if err != nil { break }
		typeData, _ := os.ReadFile(fmt.Sprintf("/sys/class/thermal/thermal_zone%d/type", i))
		var millideg float64
		fmt.Sscanf(strings.TrimSpace(string(data)), "%f", &millideg)
		if millideg == 0 { continue }
		sensor := strings.TrimSpace(string(typeData))
		if sensor == "" { sensor = fmt.Sprintf("zone%d", i) }
		temps = append(temps, Temperature{Sensor: sensor, Temp: millideg / 1000})
	}
	return temps
}

func GetCrontabs() []string {
	var lines []string
	// system crontab
	out, err := exec.Command("cat", "/etc/crontab").Output()
	if err == nil {
		for _, l := range strings.Split(string(out), "\n") {
			l = strings.TrimSpace(l)
			if l != "" && !strings.HasPrefix(l, "#") {
				lines = append(lines, l)
			}
		}
	}
	// cron.d
	entries, _ := os.ReadDir("/etc/cron.d")
	for _, e := range entries {
		out, err := exec.Command("cat", "/etc/cron.d/"+e.Name()).Output()
		if err != nil { continue }
		for _, l := range strings.Split(string(out), "\n") {
			l = strings.TrimSpace(l)
			if l != "" && !strings.HasPrefix(l, "#") {
				lines = append(lines, "["+e.Name()+"] "+l)
			}
		}
	}
	// user crontabs via crontab -l (current user)
	out2, err2 := exec.Command("crontab", "-l").Output()
	if err2 == nil {
		for _, l := range strings.Split(string(out2), "\n") {
			l = strings.TrimSpace(l)
			if l != "" && !strings.HasPrefix(l, "#") {
				lines = append(lines, "[user] "+l)
			}
		}
	}
	return lines
}

func CollectAll() MetricsSnapshot {
	return MetricsSnapshot{
		Timestamp: time.Now().Unix(),
		System:    GetSystemInfo(),
		CPU:       GetCPUStats(),
		Memory:    GetMemoryStats(),
		Disk:      GetDiskStats(),
		Network:   GetNetworkStats(),
		Temps:     GetTemperatures(),
	}
}

func formatUptime(secs uint64) string {
	d := secs / 86400; h := (secs % 86400) / 3600; m := (secs % 3600) / 60
	if d > 0 { return fmt.Sprintf("%dd %dh %dm", d, h, m) }
	return fmt.Sprintf("%dh %dm", h, m)
}
