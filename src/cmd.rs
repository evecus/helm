//! 外部命令执行（带超时），对应 Go 的 exec.CommandContext + Output/Run/CombinedOutput。

use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub struct CmdResult {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    /// combined 模式下 stdout/stderr 共用同一管道的结果（顺序与 Go 一致）
    pub combined: Vec<u8>,
    pub ok: bool,
    pub timed_out: bool,
}

fn spawn_collect(
    program: &str,
    args: &[&str],
    timeout: Duration,
    merge: bool,
) -> Result<CmdResult, String> {
    let mut cmd = Command::new(program);
    cmd.args(args);

    if merge {
        let (mut reader, writer) = os_pipe::pipe().map_err(|e| e.to_string())?;
        let writer2 = writer.try_clone().map_err(|e| e.to_string())?;
        cmd.stdout(Stdio::from(writer));
        cmd.stderr(Stdio::from(writer2));
        let mut child = cmd.spawn().map_err(|e| e.to_string())?;
        // 关闭父进程持有的写端，否则读端永远等不到 EOF
        drop(cmd);

        let handle = thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = reader.read_to_end(&mut buf);
            buf
        });

        let (mut status, mut timed_out) = (None, false);
        let start = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(s)) => {
                    status = Some(s);
                    break;
                }
                Ok(None) => {
                    if start.elapsed() >= timeout {
                        let _ = child.kill();
                        let _ = child.wait();
                        timed_out = true;
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }
        if status.is_none() && !timed_out {
            status = child.wait().ok();
        }
        let data = handle.join().unwrap_or_default();
        let ok = !timed_out && status.map(|s| s.success()).unwrap_or(false);
        Ok(CmdResult {
            stdout: Vec::new(),
            stderr: Vec::new(),
            combined: data,
            ok,
            timed_out,
        })
    } else {
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = cmd.spawn().map_err(|e| e.to_string())?;
        let mut out_pipe = child.stdout.take().unwrap();
        let mut err_pipe = child.stderr.take().unwrap();
        let h1 = thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = out_pipe.read_to_end(&mut buf);
            buf
        });
        let h2 = thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = err_pipe.read_to_end(&mut buf);
            buf
        });

        let (mut status, mut timed_out) = (None, false);
        let start = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(s)) => {
                    status = Some(s);
                    break;
                }
                Ok(None) => {
                    if start.elapsed() >= timeout {
                        let _ = child.kill();
                        let _ = child.wait();
                        timed_out = true;
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }
        if status.is_none() && !timed_out {
            status = child.wait().ok();
        }
        let stdout = h1.join().unwrap_or_default();
        let stderr = h2.join().unwrap_or_default();
        let ok = !timed_out && status.map(|s| s.success()).unwrap_or(false);
        Ok(CmdResult {
            stdout,
            stderr,
            combined: Vec::new(),
            ok,
            timed_out,
        })
    }
}

/// 对应 `.Output()`：成功返回 stdout，失败返回错误描述。
pub fn output(program: &str, args: &[&str], timeout: Duration) -> Result<Vec<u8>, String> {
    let r = spawn_collect(program, args, timeout, false)?;
    if r.ok {
        Ok(r.stdout)
    } else if r.timed_out {
        Err("command timed out".into())
    } else {
        let msg = String::from_utf8_lossy(&r.stderr).trim().to_string();
        Err(if msg.is_empty() {
            format!("{} exited with non-zero status", program)
        } else {
            msg
        })
    }
}

/// 对应 `.Run()`：只关心退出状态。
pub fn run(program: &str, args: &[&str], timeout: Duration) -> Result<(), String> {
    let r = spawn_collect(program, args, timeout, false)?;
    if r.ok {
        Ok(())
    } else if r.timed_out {
        Err("command timed out".into())
    } else {
        let msg = String::from_utf8_lossy(&r.stderr).trim().to_string();
        Err(if msg.is_empty() {
            format!("{} exited with non-zero status", program)
        } else {
            msg
        })
    }
}

/// 对应 `.CombinedOutput()`：无论成败都返回合并输出，并附带可选错误。
pub fn combined(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<(Vec<u8>, Option<String>), String> {
    let r = spawn_collect(program, args, timeout, true)?;
    let err = if r.ok {
        None
    } else if r.timed_out {
        Some("command timed out".to_string())
    } else {
        Some(format!("{} exited with non-zero status", program))
    };
    Ok((r.combined, err))
}
