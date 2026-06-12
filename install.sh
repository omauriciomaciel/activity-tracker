#!/usr/bin/env sh
set -eu

REPO="omauriciomaciel/activity-tracker"
BINARY="activity-tracker"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { printf "${GREEN}[ok]${NC} %s\n" "$*"; }
warn()  { printf "${YELLOW}[!]${NC} %s\n" "$*"; }
error() { printf "${RED}[erro]${NC} %s\n" "$*" >&2; exit 1; }

echo "=== Activity Tracker Installer ==="
echo

# ── Detect OS / arch ──────────────────────────────────────────────────────────

OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
    Linux)  OS_NAME="linux" ;;
    Darwin) OS_NAME="macos" ;;
    *)      error "OS não suportado: $OS" ;;
esac

case "$ARCH" in
    x86_64)        ARCH_NAME="x86_64" ;;
    aarch64|arm64) ARCH_NAME="aarch64" ;;
    *)             error "Arquitetura não suportada: $ARCH" ;;
esac

info "Plataforma: $OS_NAME / $ARCH_NAME"

# ── Dependencies ──────────────────────────────────────────────────────────────

command -v curl >/dev/null 2>&1 || error "curl não encontrado"

if command -v ollama >/dev/null 2>&1; then
    info "Ollama $(ollama --version 2>/dev/null || echo 'instalado')"
else
    warn "Ollama não encontrado — provider padrão não funcionará sem ele"
    warn "  Instale: curl -fsSL https://ollama.com/install.sh | sh"
    warn "  Depois:  ollama pull llama3.2"
fi

# ── Fetch latest release tag ──────────────────────────────────────────────────

info "Buscando última versão..."
LATEST_TAG=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' \
    | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')

[ -n "$LATEST_TAG" ] || error "Não foi possível obter a versão mais recente do GitHub"
info "Versão: $LATEST_TAG"

# ── Download binary ───────────────────────────────────────────────────────────

# Strip leading 'v' from tag
VERSION=$(echo "$LATEST_TAG" | sed 's/^v//')
TARBALL="activity-tracker-${VERSION}-${OS_NAME}-${ARCH_NAME}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${LATEST_TAG}/${TARBALL}"

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

info "Baixando $TARBALL..."
curl -fsSL --progress-bar "$URL" -o "$TMP_DIR/$TARBALL" \
    || error "Falha no download: $URL"

tar -xzf "$TMP_DIR/$TARBALL" -C "$TMP_DIR" activity-tracker \
    || error "Falha ao extrair $TARBALL"

# ── Install binary ────────────────────────────────────────────────────────────

mkdir -p "$INSTALL_DIR"
install -m 755 "$TMP_DIR/activity-tracker" "$INSTALL_DIR/$BINARY"
info "Binário instalado em $INSTALL_DIR/$BINARY"

# ── PATH check ────────────────────────────────────────────────────────────────

case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
        warn "$INSTALL_DIR não está no PATH"
        echo
        echo "  Adicione ao seu shell:"
        echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
        echo
        ;;
esac

# ── Autostart (launchd no macOS, systemd no Linux) ────────────────────────────

if [ "$OS" = "Darwin" ]; then
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

    # Permissions — only prompt when running interactively (not piped)
    if [ -t 0 ]; then
        echo
        echo "┌─────────────────────────────────────────────────────────────────┐"
        echo "│  Permissões necessárias no macOS (requer ação manual)           │"
        echo "├─────────────────────────────────────────────────────────────────┤"
        echo "│  1. Full Disk Access  — para ler histórico do Chrome/Brave      │"
        echo "│  2. Accessibility     — para capturar títulos de janelas        │"
        echo "└─────────────────────────────────────────────────────────────────┘"
        echo
        warn "Adicione o binário em ambas as telas: $INSTALL_DIR/$BINARY"
        echo
        echo "  Abrindo System Settings → Full Disk Access..."
        open "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles" 2>/dev/null || true
        echo "  Pressione Enter quando conceder Full Disk Access..."
        read -r _
        echo "  Abrindo System Settings → Accessibility..."
        open "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility" 2>/dev/null || true
        echo "  Pressione Enter quando conceder Accessibility..."
        read -r _
        info "Permissões configuradas"
    else
        echo
        warn "Permissões macOS — configure manualmente após a instalação:"
        warn "  System Settings → Privacy → Full Disk Access → adicionar $INSTALL_DIR/$BINARY"
        warn "  System Settings → Privacy → Accessibility   → adicionar $INSTALL_DIR/$BINARY"
    fi

elif command -v systemctl >/dev/null 2>&1; then
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
    systemctl --user start activity-tracker.service \
        || warn "Falha ao iniciar o serviço agora (iniciará no próximo login)"

    info "Serviço systemd ativado — inicia automaticamente no login"
    info "  Status: systemctl --user status activity-tracker"
    info "  Logs:   journalctl --user -u activity-tracker -f"

else
    warn "Nem systemd nem launchd encontrados — autostart não configurado"
    warn "  Inicie manualmente: $BINARY start"
fi

# ── Shell aliases ─────────────────────────────────────────────────────────────

SHELL_RC=""
if [ -f "$HOME/.zshrc" ]; then
    SHELL_RC="$HOME/.zshrc"
elif [ -f "$HOME/.bashrc" ]; then
    SHELL_RC="$HOME/.bashrc"
elif [ -f "$HOME/.bash_profile" ]; then
    SHELL_RC="$HOME/.bash_profile"
fi

ALIAS_AT="alias at='activity-tracker'"
ALIAS_ATS="alias ats='activity-tracker summary'"

if [ -n "$SHELL_RC" ]; then
    if grep -q "alias at=" "$SHELL_RC" 2>/dev/null; then
        warn "Alias 'at' já existe em $SHELL_RC — pulando"
    else
        printf '\n# activity-tracker aliases\n%s\n%s\n' "$ALIAS_AT" "$ALIAS_ATS" >> "$SHELL_RC"
        info "Aliases adicionados em $SHELL_RC"
        info "  at  → activity-tracker"
        info "  ats → activity-tracker summary"
        warn "Reinicie o shell ou execute: source $SHELL_RC"
    fi
else
    warn "Shell RC não encontrado. Adicione manualmente:"
    echo "  $ALIAS_AT"
    echo "  $ALIAS_ATS"
fi

# ── Done ──────────────────────────────────────────────────────────────────────

echo
echo "=== Instalação concluída ==="
echo
echo "Comandos:"
echo "  at start                       # inicia daemon em background"
echo "  at stop                        # para o daemon"
echo "  at status                      # verifica se está rodando"
echo "  at collect                     # coleta manual"
echo "  ats --days 3                   # resumo dos últimos 3 dias"
echo "  at tui                         # TUI interativa"
echo "  at config set-model llama3.2   # define modelo padrão"
echo
