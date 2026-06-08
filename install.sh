#!/usr/bin/env bash
set -euo pipefail

BINARY="activity-tracker"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()    { echo -e "${GREEN}[✓]${NC} $*"; }
warn()    { echo -e "${YELLOW}[!]${NC} $*"; }
error()   { echo -e "${RED}[✗]${NC} $*" >&2; exit 1; }

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

# ── Próximos passos ──────────────────────────────────────────────────────────

echo
echo "=== Instalação concluída ==="
echo
echo "Próximos passos:"
echo "  $BINARY collect                     # coleta manual"
echo "  $BINARY start                       # daemon (coleta a cada 5 min)"
echo "  $BINARY summary --days 3            # resumo dos últimos 3 dias"
echo "  $BINARY config set-model llama3.2   # define modelo padrão"
echo
