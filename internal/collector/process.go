package collector

import (
	"sort"

	"github.com/shirou/gopsutil/v3/process"
)

type ProcessInfo struct {
	PID        int32   `json:"pid"`
	Name       string  `json:"name"`
	Username   string  `json:"username"`
	CPUPercent float64 `json:"cpu_percent"`
	MemPercent float32 `json:"mem_percent"`
	MemRSS     uint64  `json:"mem_rss"`
	Status     string  `json:"status"`
	Cmdline    string  `json:"cmdline"`
}

func GetProcesses(sortBy string, sortDir string, limit int) ([]ProcessInfo, error) {
	procs, err := process.Processes()
	if err != nil {
		return nil, err
	}

	var infos []ProcessInfo
	for _, p := range procs {
		name, _ := p.Name()
		username, _ := p.Username()
		cpu, _ := p.CPUPercent()
		memPct, _ := p.MemoryPercent()
		memInfo, _ := p.MemoryInfo()
		statuses, _ := p.Status()
		cmdline, _ := p.Cmdline()

		var rss uint64
		if memInfo != nil {
			rss = memInfo.RSS
		}
		status := ""
		if len(statuses) > 0 {
			status = statuses[0]
		}

		infos = append(infos, ProcessInfo{
			PID:        p.Pid,
			Name:       name,
			Username:   username,
			CPUPercent: cpu,
			MemPercent: memPct,
			MemRSS:     rss,
			Status:     status,
			Cmdline:    cmdline,
		})
	}

	asc := sortDir == "asc"
	switch sortBy {
	case "mem":
		sort.Slice(infos, func(i, j int) bool {
			if asc { return infos[i].MemPercent < infos[j].MemPercent }
			return infos[i].MemPercent > infos[j].MemPercent
		})
	default:
		sort.Slice(infos, func(i, j int) bool {
			if asc { return infos[i].CPUPercent < infos[j].CPUPercent }
			return infos[i].CPUPercent > infos[j].CPUPercent
		})
	}

	if limit > 0 && len(infos) > limit {
		infos = infos[:limit]
	}
	return infos, nil
}

func KillProcess(pid int32) error {
	p, err := process.NewProcess(pid)
	if err != nil {
		return err
	}
	return p.Kill()
}
