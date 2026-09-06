package collector

import (
	"context"
	"fmt"
	"os"
	"os/exec"
	"sort"
	"strconv"
	"strings"
	"time"
)

type SystemdService struct {
	Unit             string `json:"unit"`
	Load             string `json:"load"`
	Active           string `json:"active"`
	Sub              string `json:"sub"`
	Description      string `json:"description"`
	// 详细信息（通过 systemctl show 获取）
	PID              string `json:"pid"`
	MemoryCurrent    uint64 `json:"memory_bytes"`
	MemoryFormatted  string `json:"memory"`
	CPUUsageNsec     uint64 `json:"cpu_ns"`
	CPUFormatted     string `json:"cpu_time"`
	UnitFileState    string `json:"unit_file_state"`
	MainPID          string `json:"main_pid"`
	ExecStart        string `json:"exec_start"`
	FragmentPath     string `json:"fragment_path"`
	StartedAt        string `json:"started_at"`
	TasksCurrent     string `json:"tasks"`
}

func GetServices() ([]SystemdService, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	out, err := exec.CommandContext(ctx, "systemctl", "list-units",
		"--type=service", "--all", "--no-pager", "--plain", "--no-legend").Output()
	if err != nil {
		return nil, err
	}

	var services []SystemdService
	for _, line := range strings.Split(string(out), "\n") {
		fields := strings.Fields(line)
		if len(fields) < 4 {
			continue
		}
		svc := SystemdService{
			Unit:   fields[0],
			Load:   fields[1],
			Active: fields[2],
			Sub:    fields[3],
		}
		if len(fields) > 4 {
			svc.Description = strings.Join(fields[4:], " ")
		}
		// 只对 active 服务获取详细信息（避免超时）
		if svc.Active == "active" {
			enrichService(&svc)
		} else {
			// 非运行服务只获取 unit_file_state
			getUnitFileState(&svc)
		}
		services = append(services, svc)
	}
	return services, nil
}

func enrichService(svc *SystemdService) {
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()

	props := []string{
		"MainPID", "MemoryCurrent", "CPUUsageNSec",
		"UnitFileState", "ExecStart", "FragmentPath",
		"TasksCurrent", "ActiveEnterTimestamp",
	}
	args := append([]string{"show", svc.Unit, "--no-pager"}, propsArgs(props)...)
	out, err := exec.CommandContext(ctx, "systemctl", args...).Output()
	if err != nil {
		return
	}

	kv := parseKV(string(out))

	svc.MainPID = kv["MainPID"]
	svc.UnitFileState = kv["UnitFileState"]
	svc.FragmentPath = kv["FragmentPath"]
	svc.TasksCurrent = kv["TasksCurrent"]

	// ExecStart 只取路径部分
	if es := kv["ExecStart"]; es != "" {
		if idx := strings.Index(es, "path="); idx >= 0 {
			rest := es[idx+5:]
			if end := strings.IndexAny(rest, " ;"); end >= 0 {
				svc.ExecStart = rest[:end]
			} else {
				svc.ExecStart = rest
			}
		}
	}

	// 内存
	if mem := kv["MemoryCurrent"]; mem != "" && mem != "[not set]" && mem != "18446744073709551615" {
		if v, err := strconv.ParseUint(mem, 10, 64); err == nil {
			svc.MemoryCurrent = v
			svc.MemoryFormatted = formatBytes(v)
		}
	}

	// CPU 时间
	if cpu := kv["CPUUsageNSec"]; cpu != "" && cpu != "[not set]" {
		if v, err := strconv.ParseUint(cpu, 10, 64); err == nil {
			svc.CPUUsageNsec = v
			svc.CPUFormatted = formatNsec(v)
		}
	}

	// 启动时间
	if ts := kv["ActiveEnterTimestamp"]; ts != "" && ts != "0" {
		svc.StartedAt = ts
	}
}

func getUnitFileState(svc *SystemdService) {
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()
	out, err := exec.CommandContext(ctx, "systemctl", "show", svc.Unit,
		"--no-pager", "--property=UnitFileState", "--property=FragmentPath", "--property=ExecStart").Output()
	if err != nil {
		return
	}
	kv := parseKV(string(out))
	svc.UnitFileState = kv["UnitFileState"]
	svc.FragmentPath = kv["FragmentPath"]
	if es := kv["ExecStart"]; es != "" {
		if idx := strings.Index(es, "path="); idx >= 0 {
			rest := es[idx+5:]
			if end := strings.IndexAny(rest, " ;"); end >= 0 {
				svc.ExecStart = rest[:end]
			} else {
				svc.ExecStart = rest
			}
		}
	}
}

func propsArgs(props []string) []string {
	var args []string
	for _, p := range props {
		args = append(args, "--property="+p)
	}
	return args
}

func parseKV(s string) map[string]string {
	m := make(map[string]string)
	for _, line := range strings.Split(s, "\n") {
		if idx := strings.IndexByte(line, '='); idx > 0 {
			m[line[:idx]] = line[idx+1:]
		}
	}
	return m
}

func formatBytes(b uint64) string {
	switch {
	case b >= 1<<30:
		return fmt.Sprintf("%.1f GB", float64(b)/(1<<30))
	case b >= 1<<20:
		return fmt.Sprintf("%.1f MB", float64(b)/(1<<20))
	case b >= 1<<10:
		return fmt.Sprintf("%.1f KB", float64(b)/(1<<10))
	default:
		return fmt.Sprintf("%d B", b)
	}
}

func formatNsec(ns uint64) string {
	d := time.Duration(ns)
	switch {
	case d >= time.Hour:
		return fmt.Sprintf("%.1fh", d.Hours())
	case d >= time.Minute:
		return fmt.Sprintf("%.1fm", d.Minutes())
	default:
		return fmt.Sprintf("%.1fs", d.Seconds())
	}
}

func ServiceAction(unit, action string) error {
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	return exec.CommandContext(ctx, "systemctl", action, unit).Run()
}

func GetServiceLogs(unit string, lines int) (string, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	out, err := exec.CommandContext(ctx, "journalctl", "-u", unit,
		"-n", strconv.Itoa(lines), "--no-pager", "--output=short-iso").CombinedOutput()
	return string(out), err
}

func SortServices(svcs []SystemdService, sortBy, dir string) {
	if sortBy == "" { return }
	sort.Slice(svcs, func(i, j int) bool {
		var vi, vj uint64
		switch sortBy {
		case "memory":
			vi, vj = svcs[i].MemoryCurrent, svcs[j].MemoryCurrent
		case "cpu":
			vi, vj = svcs[i].CPUUsageNsec, svcs[j].CPUUsageNsec
		default:
			return false
		}
		if dir == "asc" { return vi < vj }
		return vi > vj
	})
}

func ReadServiceFile(unit string) (string, string, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	out, err := exec.CommandContext(ctx, "systemctl", "show", unit, "--no-pager", "--property=FragmentPath").Output()
	if err != nil { return "", "", err }
	kv := parseKV(string(out))
	path := kv["FragmentPath"]
	if path == "" { return "", "", fmt.Errorf("no FragmentPath for unit %s", unit) }
	content, err := os.ReadFile(path)
	if err != nil { return "", "", err }
	return string(content), path, nil
}

func WriteServiceFile(unit, content string) error {
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	out, err := exec.CommandContext(ctx, "systemctl", "show", unit, "--no-pager", "--property=FragmentPath,ActiveState").Output()
	if err != nil { return err }
	kv := parseKV(string(out))
	path := kv["FragmentPath"]
	if path == "" { return fmt.Errorf("no FragmentPath for %s", unit) }
	wasActive := kv["ActiveState"] == "active"

	if err := os.WriteFile(path, []byte(content), 0644); err != nil { return err }

	ctx2, cancel2 := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel2()
	if err := exec.CommandContext(ctx2, "systemctl", "daemon-reload").Run(); err != nil { return err }

	if wasActive {
		ctx3, cancel3 := context.WithTimeout(context.Background(), 30*time.Second)
		defer cancel3()
		exec.CommandContext(ctx3, "systemctl", "restart", unit).Run()
	}
	return nil
}
