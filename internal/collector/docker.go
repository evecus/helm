package collector

import (
	"context"
	"encoding/json"
	"fmt"
	"os/exec"
	"strconv"
	"strings"
	"time"
)

type Container struct {
	ID      string  `json:"id"`
	Name    string  `json:"name"`
	Image   string  `json:"image"`
	Status  string  `json:"status"`
	State   string  `json:"state"`
	Ports   string  `json:"ports"`
	Created string  `json:"created"`
	CPU     float64 `json:"cpu_percent"`
	MemPct  float64 `json:"mem_percent"`
	MemUsed uint64  `json:"mem_used"`
	MemLim  uint64  `json:"mem_limit"`
}

func GetContainers() ([]Container, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	out, err := exec.CommandContext(ctx, "docker", "ps", "-a",
		"--format", `{"id":"{{.ID}}","name":"{{.Names}}","image":"{{.Image}}","status":"{{.Status}}","state":"{{.State}}","ports":"{{.Ports}}","created":"{{.CreatedAt}}"}`).Output()
	if err != nil {
		return nil, fmt.Errorf("docker not available: %w", err)
	}

	var containers []Container
	for _, line := range strings.Split(strings.TrimSpace(string(out)), "\n") {
		if line == "" {
			continue
		}
		var c Container
		if err := json.Unmarshal([]byte(line), &c); err == nil {
			containers = append(containers, c)
		}
	}

	// Enrich with stats (non-blocking)
	ctx2, cancel2 := context.WithTimeout(context.Background(), 8*time.Second)
	defer cancel2()
	statsOut, err := exec.CommandContext(ctx2, "docker", "stats", "--no-stream",
		"--format", `{{.ID}}\t{{.CPUPerc}}\t{{.MemPerc}}\t{{.MemUsage}}`).Output()
	if err == nil {
		type stat struct{ cpu, memPct float64; memUsed, memLim uint64 }
		statsMap := make(map[string]stat)
		for _, line := range strings.Split(strings.TrimSpace(string(statsOut)), "\n") {
			parts := strings.Split(line, "\t")
			if len(parts) < 4 {
				continue
			}
			id := parts[0]
			cpu, _ := strconv.ParseFloat(strings.TrimSuffix(parts[1], "%"), 64)
			memPct, _ := strconv.ParseFloat(strings.TrimSuffix(parts[2], "%"), 64)
			// parse mem usage like "128MiB / 2GiB"
			var used, lim uint64
			memParts := strings.Split(parts[3], " / ")
			if len(memParts) == 2 {
				used = parseMemStr(memParts[0])
				lim = parseMemStr(memParts[1])
			}
			statsMap[id] = stat{cpu, memPct, used, lim}
		}
		for i, c := range containers {
			if s, ok := statsMap[c.ID]; ok {
				containers[i].CPU = s.cpu
				containers[i].MemPct = s.memPct
				containers[i].MemUsed = s.memUsed
				containers[i].MemLim = s.memLim
			}
		}
	}
	return containers, nil
}

func parseMemStr(s string) uint64 {
	s = strings.TrimSpace(s)
	multipliers := map[string]uint64{
		"B": 1, "KiB": 1024, "MiB": 1024 * 1024, "GiB": 1024 * 1024 * 1024,
		"KB": 1000, "MB": 1000 * 1000, "GB": 1000 * 1000 * 1000,
	}
	for suffix, mult := range multipliers {
		if strings.HasSuffix(s, suffix) {
			val, _ := strconv.ParseFloat(strings.TrimSuffix(s, suffix), 64)
			return uint64(val * float64(mult))
		}
	}
	val, _ := strconv.ParseUint(s, 10, 64)
	return val
}

func ContainerAction(id, action string) error {
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	return exec.CommandContext(ctx, "docker", action, id).Run()
}

func GetContainerLogs(id string, lines int) (string, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	out, err := exec.CommandContext(ctx, "docker", "logs", "--tail", strconv.Itoa(lines), id).CombinedOutput()
	return string(out), err
}

// InspectResult holds detailed container info
type InspectResult struct {
	Image         string   `json:"image"`
	Status        string   `json:"status"`
	Created       string   `json:"created"`
	RestartPolicy string   `json:"restart_policy"`
	Env           []string `json:"env"`
	Mounts        []string `json:"mounts"`
	Networks      []string `json:"networks"`
	Ports         string   `json:"ports"`
	Cmd           []string `json:"cmd"`
	ComposeFile   string   `json:"compose_file"`
}

func InspectContainer(id string) (*InspectResult, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	// docker inspect always returns a JSON array
	out, err := exec.CommandContext(ctx, "docker", "inspect", id).Output()
	if err != nil {
		return nil, fmt.Errorf("docker inspect failed: %w", err)
	}

	var raw []map[string]interface{}
	if err := json.Unmarshal(out, &raw); err != nil || len(raw) == 0 {
		return nil, fmt.Errorf("parse error: %v (output: %.300s)", err, string(out))
	}
	c := raw[0]

	result := &InspectResult{}
	result.Env = []string{}
	result.Mounts = []string{}
	result.Networks = []string{}
	result.Cmd = []string{}

	if cfg, ok := c["Config"].(map[string]interface{}); ok {
		result.Image, _ = cfg["Image"].(string)
		if env, ok := cfg["Env"].([]interface{}); ok {
			for _, e := range env {
				if s, ok := e.(string); ok {
					result.Env = append(result.Env, s)
				}
			}
		}
		if cmd, ok := cfg["Cmd"].([]interface{}); ok {
			for _, v := range cmd {
				if s, ok := v.(string); ok {
					result.Cmd = append(result.Cmd, s)
				}
			}
		}
		// Check compose labels
		if labels, ok := cfg["Labels"].(map[string]interface{}); ok {
			if wdir, ok := labels["com.docker.compose.project.working_dir"].(string); ok && wdir != "" {
				candidates := []string{
					wdir + "/docker-compose.yml",
					wdir + "/docker-compose.yaml",
					wdir + "/compose.yml",
					wdir + "/compose.yaml",
				}
				for _, p := range candidates {
					if fileExists(p) {
						result.ComposeFile = p
						break
					}
				}
				if result.ComposeFile == "" {
					result.ComposeFile = wdir + "/docker-compose.yml"
				}
			}
		}
	}

	if state, ok := c["State"].(map[string]interface{}); ok {
		result.Status, _ = state["Status"].(string)
	}
	result.Created, _ = c["Created"].(string)

	if hc, ok := c["HostConfig"].(map[string]interface{}); ok {
		if rp, ok := hc["RestartPolicy"].(map[string]interface{}); ok {
			result.RestartPolicy, _ = rp["Name"].(string)
		}
		// Build ports string from PortBindings
		if pb, ok := hc["PortBindings"].(map[string]interface{}); ok {
			var parts []string
			for containerPort, bindings := range pb {
				if bList, ok := bindings.([]interface{}); ok {
					for _, b := range bList {
						if bm, ok := b.(map[string]interface{}); ok {
							hostPort, _ := bm["HostPort"].(string)
							hostIP, _ := bm["HostIp"].(string)
							if hostIP == "" {
								hostIP = "0.0.0.0"
							}
							parts = append(parts, fmt.Sprintf("%s:%s->%s", hostIP, hostPort, containerPort))
						}
					}
				}
			}
			result.Ports = strings.Join(parts, ", ")
		}
	}

	if mounts, ok := c["Mounts"].([]interface{}); ok {
		for _, m := range mounts {
			if mp, ok := m.(map[string]interface{}); ok {
				src, _ := mp["Source"].(string)
				dst, _ := mp["Destination"].(string)
				result.Mounts = append(result.Mounts, src+"->"+dst)
			}
		}
	}

	if netSettings, ok := c["NetworkSettings"].(map[string]interface{}); ok {
		if networks, ok := netSettings["Networks"].(map[string]interface{}); ok {
			for name := range networks {
				result.Networks = append(result.Networks, name)
			}
		}
	}

	return result, nil
}

// fileExists checks whether a file path exists
func fileExists(path string) bool {
	out, err := exec.Command("bash", "-c", fmt.Sprintf("test -f '%s' && echo yes", path)).Output()
	return err == nil && strings.TrimSpace(string(out)) == "yes"
}

func ReadComposeFile(path string) (string, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	out, err := exec.CommandContext(ctx, "cat", path).Output()
	if err != nil {
		return "", fmt.Errorf("cannot read file: %w", err)
	}
	return string(out), nil
}

func WriteAndApplyCompose(path, content, containerID string) (string, error) {
	// Write updated compose file
	ctx, cancel := context.WithTimeout(context.Background(), 60*time.Second)
	defer cancel()

	// Write the file
	writeCmd := exec.Command("bash", "-c", fmt.Sprintf("cat > %s", path))
	writeCmd.Stdin = strings.NewReader(content)
	if out, err := writeCmd.CombinedOutput(); err != nil {
		return "", fmt.Errorf("write failed: %s %w", string(out), err)
	}

	// Get working directory from path
	dir := path[:strings.LastIndex(path, "/")]
	if dir == "" {
		dir = "."
	}

	// docker compose down then up
	downOut, _ := exec.CommandContext(ctx, "docker", "compose", "-f", path, "down").CombinedOutput()
	upOut, err := exec.CommandContext(ctx, "docker", "compose", "-f", path, "up", "-d").CombinedOutput()
	log := fmt.Sprintf("=== Working dir: %s ===\n\n=== docker compose down ===\n%s\n=== docker compose up -d ===\n%s", dir, string(downOut), string(upOut))
	if err != nil {
		return log, fmt.Errorf("compose up failed: %s", string(upOut))
	}
	return log, nil
}

func PullAndUpdateContainer(id string) (string, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 300*time.Second)
	defer cancel()

	// Get image name
	imgOut, err := exec.CommandContext(ctx, "docker", "inspect", "--format", "{{.Config.Image}}", id).Output()
	if err != nil {
		return "", fmt.Errorf("cannot get image: %w", err)
	}
	image := strings.TrimSpace(string(imgOut))

	var logBuf strings.Builder
	logBuf.WriteString(fmt.Sprintf("=== Pulling image: %s ===\n", image))

	pullOut, _ := exec.CommandContext(ctx, "docker", "pull", image).CombinedOutput()
	logBuf.Write(pullOut)

	// Check if compose container
	labelsOut, _ := exec.CommandContext(ctx, "docker", "inspect",
		"--format", "{{index .Config.Labels \"com.docker.compose.project.working_dir\"}}", id).Output()
	wdir := strings.TrimSpace(string(labelsOut))

	if wdir != "" {
		logBuf.WriteString(fmt.Sprintf("\n=== Detected docker-compose project at: %s ===\n", wdir))
		// Find compose file
		var composePath string
		for _, name := range []string{"docker-compose.yml", "docker-compose.yaml", "compose.yml", "compose.yaml"} {
			p := wdir + "/" + name
			if _, err := exec.Command("test", "-f", p).Output(); err == nil {
				composePath = p
				break
			}
		}
		if composePath == "" {
			composePath = wdir + "/docker-compose.yml"
		}
		logBuf.WriteString(fmt.Sprintf("Compose file: %s\n\n", composePath))

		downOut, _ := exec.CommandContext(ctx, "docker", "compose", "-f", composePath, "down").CombinedOutput()
		logBuf.WriteString("=== docker compose down ===\n")
		logBuf.Write(downOut)

		upOut, err2 := exec.CommandContext(ctx, "docker", "compose", "-f", composePath, "up", "-d").CombinedOutput()
		logBuf.WriteString("\n=== docker compose up -d ===\n")
		logBuf.Write(upOut)
		if err2 != nil {
			return logBuf.String(), fmt.Errorf("compose up failed")
		}
	} else {
		// Plain docker container - get run args and recreate
		logBuf.WriteString(fmt.Sprintf("\n=== Recreating container: %s ===\n", id))
		stopOut, _ := exec.CommandContext(ctx, "docker", "stop", id).CombinedOutput()
		logBuf.Write(stopOut)
		rmOut, _ := exec.CommandContext(ctx, "docker", "rm", id).CombinedOutput()
		logBuf.Write(rmOut)
		logBuf.WriteString("Container stopped and removed. Please restart manually or redeploy with original run command.")
	}

	return logBuf.String(), nil
}
