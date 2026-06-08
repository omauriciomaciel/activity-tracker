use crate::{collector, config};
use anyhow::{Context, Result};
use tokio::signal;
use tokio::time::{self, Duration};

pub fn pid_file() -> std::path::PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("activity-tracker")
        .join("daemon.pid")
}

pub async fn run(interval_min: u64, foreground: bool) -> Result<()> {
    if !foreground {
        spawn_background(interval_min)?;
        return Ok(());
    }
    run_foreground(interval_min).await
}

fn spawn_background(interval_min: u64) -> Result<()> {
    use std::os::unix::process::CommandExt;

    let exe = std::env::current_exe().context("não foi possível obter caminho do binário")?;

    let child = std::process::Command::new(exe)
        .args([
            "start",
            "--foreground",
            "--interval",
            &interval_min.to_string(),
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .process_group(0)
        .spawn()
        .context("falha ao iniciar daemon em background")?;

    println!("Daemon iniciado (PID {})", child.id());
    println!("  Logs:  {}", config::log_dir().display());
    println!("  Parar: activity-tracker stop");
    Ok(())
}

async fn run_foreground(interval_min: u64) -> Result<()> {
    let log_dir = config::log_dir();

    let pid_path = pid_file();
    if let Some(parent) = pid_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&pid_path, std::process::id().to_string())?;
    let _guard = PidGuard(pid_path);

    let mut tick = time::interval(Duration::from_secs(interval_min * 60));

    do_collect(log_dir.clone()).await;

    loop {
        tokio::select! {
            _ = tick.tick() => { do_collect(log_dir.clone()).await; }
            _ = signal::ctrl_c() => { break; }
        }
    }

    Ok(())
}

struct PidGuard(std::path::PathBuf);
impl Drop for PidGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

pub fn stop() -> Result<()> {
    let pid_path = pid_file();

    if !pid_path.exists() {
        println!("Daemon não está rodando.");
        return Ok(());
    }

    let pid_str = std::fs::read_to_string(&pid_path)?;
    let pid: u32 = pid_str.trim().parse().context("PID inválido no arquivo")?;

    let ok = std::process::Command::new("kill")
        .arg(pid.to_string())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if ok {
        println!("Daemon (PID {pid}) parado.");
    } else {
        println!("Não foi possível parar PID {pid} — pode já ter encerrado.");
    }
    let _ = std::fs::remove_file(&pid_path);

    Ok(())
}

pub fn status() -> Result<()> {
    let pid_path = pid_file();

    if !pid_path.exists() {
        println!("Daemon: parado");
        return Ok(());
    }

    let pid_str = std::fs::read_to_string(&pid_path)?;
    let pid: u32 = pid_str.trim().parse().unwrap_or(0);

    if std::path::Path::new(&format!("/proc/{pid}")).exists() {
        println!("Daemon: rodando (PID {pid})");
        println!("  Logs: {}", config::log_dir().display());
    } else {
        println!("Daemon: parado (PID file obsoleto removido)");
        let _ = std::fs::remove_file(&pid_path);
    }

    Ok(())
}

async fn do_collect(log_dir: std::path::PathBuf) {
    let now = chrono::Local::now().format("%H:%M:%S").to_string();
    let result = tokio::task::spawn_blocking(move || collector::collect_all(&log_dir)).await;
    match result {
        Ok(Ok(n)) => eprintln!("[{now}] {n} entradas coletadas"),
        Ok(Err(e)) => eprintln!("[{now}] Erro na coleta: {e}"),
        Err(e) => eprintln!("[{now}] Erro interno: {e}"),
    }
}
