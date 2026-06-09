#!/usr/bin/env bash
set -euo pipefail

BINARY="activity-tracker"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()    { echo -e "${GREEN}[ok]${NC} $*"; }
warn()    { echo -e "${YELLOW}[!]${NC} $*"; }
error()   { echo -e "${RED}[erro]${NC} $*" >&2; exit 1; }

echo "=== Activity Tracker Installer ==="
echo

# ── Dependências obrigatórias ────────────────────────────────────────────────

if ! command -v cargo &>/dev/null; then
    error "Rust/Cargo não encontrado. Instale em https://rustup.rs e tente novamente."
fi
info "Cargo $(cargo --version)"

if ! command -v ollama &>/dev/null; then
    warn "Ollama não encontrado. O comando 'summary' não funcionará sem ele."
    warn "Instale em https://ollama.com e execute: ollama pull llama3.2"
else
    info "Ollama $(ollama --version 2>/dev/null || echo 'instalado')"
fi

# ── Dependências opcionais ───────────────────────────────────────────────────

if command -v wmctrl &>/dev/null; then
    info "wmctrl encontrado — captura de janelas X11 ativada"
else
    warn "wmctrl não encontrado. Instale com: sudo apt install wmctrl  (opcional)"
fi

# ── Build ────────────────────────────────────────────────────────────────────

echo
echo "Compilando em modo release..."
cargo build --release --manifest-path "$(dirname "$0")/Cargo.toml" 2>&1

SRC="$(dirname "$0")/target/release/$BINARY"
[[ -f "$SRC" ]] || error "Build falhou — binário não encontrado em $SRC"
info "Build concluído"

# ── Instalação ───────────────────────────────────────────────────────────────

mkdir -p "$INSTALL_DIR"
rm -f "$INSTALL_DIR/$BINARY"
cp "$SRC" "$INSTALL_DIR/$BINARY"
chmod +x "$INSTALL_DIR/$BINARY"
info "Binário instalado em $INSTALL_DIR/$BINARY"

# ── PATH check ───────────────────────────────────────────────────────────────

if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    warn "$INSTALL_DIR não está no PATH."
    echo
    echo "Adicione ao seu shell:"
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo
    echo "Ou copie para /usr/local/bin (requer sudo):"
    echo "  sudo cp $SRC /usr/local/bin/$BINARY"
fi

# ── Autostart (systemd no Linux, launchd no macOS) ───────────────────────────

if [[ "$(uname)" == "Darwin" ]]; then
    echo
    echo "Configurando autostart via launchd (macOS)..."

    PLIST_DIR="$HOME/Library/LaunchAgents"
    PLIST_FILE="$PLIST_DIR/com.activity-tracker.plist"
    mkdir -p "$PLIST_DIR"
    mkdir -p "$HOME/.local/share/activity-tracker"

    cat > "$PLIST_FILE" << PLIST_END
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.activity-tracker</string>
    <key>ProgramArguments</key>
    <array>
        <string>$INSTALL_DIR/$BINARY</string>
        <string>start</string>
        <string>--foreground</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>StandardOutPath</key>
    <string>$HOME/.local/share/activity-tracker/daemon.log</string>
    <key>StandardErrorPath</key>
    <string>$HOME/.local/share/activity-tracker/daemon.log</string>
</dict>
</plist>
PLIST_END

    launchctl unload "$PLIST_FILE" 2>/dev/null || true
    launchctl load -w "$PLIST_FILE"

    info "LaunchAgent ativado — inicia automaticamente no login"
    info "  Status: launchctl list | grep activity-tracker"
    info "  Parar:  launchctl unload $PLIST_FILE"
    info "  Logs:   tail -f $HOME/.local/share/activity-tracker/daemon.log"

    # ── Permissões macOS ─────────────────────────────────────────────────────
    echo
    echo "┌─────────────────────────────────────────────────────────────────┐"
    echo "│  Permissões necessárias no macOS (requer ação manual)           │"
    echo "├─────────────────────────────────────────────────────────────────┤"
    echo "│  1. Full Disk Access  — para ler histórico do Chrome/Brave      │"
    echo "│  2. Accessibility     — para capturar títulos de janelas        │"
    echo "└─────────────────────────────────────────────────────────────────┘"
    echo
    warn "Abra as telas abaixo e adicione: $INSTALL_DIR/$BINARY"
    echo
    echo "  Abrindo System Settings → Full Disk Access..."
    open "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles" 2>/dev/null || true
    echo "  Pressione Enter quando conceder Full Disk Access..."
    read -r
    echo "  Abrindo System Settings → Accessibility..."
    open "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility" 2>/dev/null || true
    echo "  Pressione Enter quando conceder Accessibility..."
    read -r
    info "Permissões configuradas"

elif command -v systemctl &>/dev/null; then
    echo
    echo "Configurando autostart via systemd..."

    SYSTEMD_DIR="$HOME/.config/systemd/user"
    SERVICE_FILE="$SYSTEMD_DIR/activity-tracker.service"
    mkdir -p "$SYSTEMD_DIR"

    cat > "$SERVICE_FILE" << SYSTEMD_END
[Unit]
Description=Activity Tracker daemon
After=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=simple
ExecStart=$INSTALL_DIR/$BINARY start --foreground
Restart=on-failure
RestartSec=30
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
SYSTEMD_END

    systemctl --user daemon-reload
    systemctl --user enable activity-tracker.service
    systemctl --user start activity-tracker.service || warn "Falha ao iniciar o serviço agora (será iniciado no próximo login)"

    info "Serviço systemd ativado — inicia automaticamente no login"
    info "  Status: systemctl --user status activity-tracker"
    info "  Logs:   journalctl --user -u activity-tracker -f"

else
    warn "Nem systemd nem launchd encontrados — autostart não configurado."
    warn "Inicie manualmente com: $BINARY start"
fi


# ── Próximos passos ──────────────────────────────────────────────────────────

echo
echo "=== Instalação concluída ==="
echo
echo "Comandos:"
echo "  $BINARY start                       # inicia daemon em background"
echo "  $BINARY stop                        # para o daemon"
echo "  $BINARY status                      # verifica se está rodando"
echo "  $BINARY collect                     # coleta manual"
echo "  $BINARY summary --days 3            # resumo dos últimos 3 dias"
echo "  $BINARY config set-model llama3.2   # define modelo padrão"
echo
