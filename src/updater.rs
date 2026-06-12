use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::env;
use std::io::Read;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const REPO_URL: &str = env!("CARGO_PKG_REPOSITORY");

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

pub fn run() -> Result<()> {
    let repo_path = REPO_URL
        .trim_end_matches('/')
        .trim_start_matches("https://github.com/");

    let api_url = format!("https://api.github.com/repos/{}/releases/latest", repo_path);

    println!("Versão atual: v{}", CURRENT_VERSION);
    println!("Consultando GitHub...");

    let client = reqwest::blocking::Client::builder()
        .user_agent(format!("activity-tracker/{}", CURRENT_VERSION))
        .build()?;

    let release: Release = client
        .get(&api_url)
        .send()
        .context("Falha ao contatar GitHub API")?
        .error_for_status()
        .context("GitHub API retornou erro")?
        .json()
        .context("Falha ao parsear resposta da API")?;

    let latest = release.tag_name.trim_start_matches('v');

    if latest == CURRENT_VERSION {
        println!("Já está na versão mais recente: v{}", CURRENT_VERSION);
        return Ok(());
    }

    println!(
        "Nova versão disponível: v{} (atual: v{})",
        latest, CURRENT_VERSION
    );

    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    let asset_name = match os {
        "linux" => format!("activity-tracker-{}-linux-{}.tar.gz", latest, arch),
        "macos" => format!("activity-tracker-{}-macos-{}.tar.gz", latest, arch),
        "windows" => format!("activity-tracker-{}-windows-{}.zip", latest, arch),
        other => bail!("Plataforma não suportada: {}", other),
    };

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .with_context(|| {
            format!(
                "Asset '{}' não encontrado na release v{}",
                asset_name, latest
            )
        })?;

    println!("Baixando {}...", asset_name);

    let bytes = client
        .get(&asset.browser_download_url)
        .send()
        .context("Falha ao baixar asset")?
        .error_for_status()
        .context("Servidor retornou erro no download")?
        .bytes()
        .context("Falha ao ler bytes do download")?;

    let binary_name = if os == "windows" {
        "activity-tracker.exe"
    } else {
        "activity-tracker"
    };

    let binary_bytes = if asset_name.ends_with(".tar.gz") {
        extract_from_targz(&bytes, binary_name)?
    } else {
        extract_from_zip(&bytes, binary_name)?
    };

    let dest = env::current_exe().context("Não foi possível determinar o binário atual")?;
    let dest = std::fs::canonicalize(&dest).unwrap_or(dest);

    // Write to tmp on same filesystem → atomic rename (no window without binary)
    let tmp = dest.with_extension("_new");
    std::fs::write(&tmp, &binary_bytes)
        .with_context(|| format!("Falha ao escrever em {}", tmp.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))
            .context("Falha ao definir permissões")?;
    }

    std::fs::rename(&tmp, &dest)
        .with_context(|| format!("Falha ao substituir {}", dest.display()))?;

    println!("\nAtualizado para v{} → {}", latest, dest.display());
    Ok(())
}

fn extract_from_targz(data: &[u8], filename: &str) -> Result<Vec<u8>> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let gz = GzDecoder::new(data);
    let mut archive = Archive::new(gz);

    for entry in archive.entries().context("Falha ao ler arquivo tar")? {
        let mut entry = entry.context("Entrada inválida no tar")?;
        let path = entry.path().context("Path inválido no tar")?;
        if path.file_name().and_then(|n| n.to_str()) == Some(filename) {
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .context("Falha ao ler entrada")?;
            return Ok(buf);
        }
    }

    bail!("Binário '{}' não encontrado no .tar.gz", filename)
}

fn extract_from_zip(data: &[u8], filename: &str) -> Result<Vec<u8>> {
    use std::io::Cursor;
    use zip::ZipArchive;

    let cursor = Cursor::new(data);
    let mut archive = ZipArchive::new(cursor).context("Falha ao abrir arquivo zip")?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).context("Entrada inválida no zip")?;
        if file.name().ends_with(filename) {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).context("Falha ao ler entrada")?;
            return Ok(buf);
        }
    }

    bail!("Binário '{}' não encontrado no .zip", filename)
}
