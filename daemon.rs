use crate::{collector, config};
use anyhow::Result;
use tokio::signal;
use tokio::time::{self, Duration};

pub async fn run(interval_min: u64) -> Result<()> {
    let log_dir = config::log_dir();

    println!("🚀 Activity Tracker daemon iniciado");
    println!("   Intervalo: {}min", interval_min);
    println!("   Logs:      {}", log_dir.display());
    println!("   Parar:     Ctrl+C\n");

    let mut tick = time::interval(Duration::from_secs(interval_min * 60));

    // Primeira coleta imediata
    do_collect(&log_dir);

    loop {
        tokio::select! {
            _ = tick.tick() => {
                do_collect(&log_dir);
            }
            _ = signal::ctrl_c() => {
                println!("\n⏹  Daemon parado.");
                break;
            }
        }
    }

    Ok(())
}

fn do_collect(log_dir: &std::path::Path) {
    let now = chrono::Local::now().format("%H:%M:%S");
    match collector::collect_all(log_dir) {
        Ok(n) => println!("[{now}] ✅ {n} entradas coletadas"),
        Err(e) => eprintln!("[{now}] ❌ Erro na coleta: {e}"),
    }
}
