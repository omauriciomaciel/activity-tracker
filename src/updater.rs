use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

const SOURCE_DIR: &str = env!("CARGO_MANIFEST_DIR");

pub fn run() -> Result<()> {
    let src = Path::new(SOURCE_DIR);

    println!("Repositório: {}", src.display());

    // ── git pull ─────────────────────────────────────────────────────────────

    // Warn if remote uses unencrypted HTTP
    if let Ok(out) = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(src)
        .output()
    {
        let remote = String::from_utf8_lossy(&out.stdout);
        if remote.trim_start().starts_with("http://") {
            eprintln!("Aviso: remote 'origin' usa HTTP sem criptografia — considere HTTPS ou SSH");
        }
    }

    println!("\n[1/3] git pull...");
    let status = Command::new("git")
        .args(["pull", "--ff-only"])
        .current_dir(src)
        .status()
        .context("Falha ao executar git pull")?;

    if !status.success() {
        bail!("git pull falhou — resolva conflitos manualmente e tente novamente");
    }

    // ── cargo build --release ─────────────────────────────────────────────────

    println!("\n[2/3] Compilando...");
    let status = Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(src)
        .status()
        .context("Falha ao executar cargo build")?;

    if !status.success() {
        bail!("Compilação falhou");
    }

    // ── instala o binário ─────────────────────────────────────────────────────

    println!("\n[3/3] Instalando...");

    let new_bin = src.join("target/release/activity-tracker");
    if !new_bin.exists() {
        bail!("Binário compilado não encontrado em {}", new_bin.display());
    }

    let dest = std::env::current_exe()
        .context("Não foi possível determinar o caminho do binário atual")?;
    let dest = std::fs::canonicalize(&dest).unwrap_or(dest);

    // Copia para temp na mesma partição, depois rename atômico — evita janela sem binário
    let tmp_dest = dest.with_extension("_new");
    std::fs::copy(&new_bin, &tmp_dest)
        .with_context(|| format!("Falha ao copiar para {}", tmp_dest.display()))?;
    std::fs::rename(&tmp_dest, &dest)
        .with_context(|| format!("Falha ao substituir {}", dest.display()))?;

    println!("\nAtualização concluída → {}", dest.display());
    Ok(())
}
