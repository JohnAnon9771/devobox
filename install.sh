#!/usr/bin/env bash
set -euo pipefail

# Cores
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

# URLs
DISTROBOX_INI_URL="https://raw.githubusercontent.com/JohnAnon9771/devobox/main/distrobox.ini"
DEVOBOX_DIR="$HOME/devobox"
DEVOBIN_DIR="$HOME/.local/bin"
DEVOBIN_SCRIPT="$DEVOBIN_DIR/devobox"

log_info() { echo -e "${BLUE}ℹ${NC} $1"; }
log_success() { echo -e "${GREEN}✓${NC} $1"; }
log_warn() { echo -e "${YELLOW}⚠${NC} $1"; }
log_error() { echo -e "${RED}✗${NC} $1"; }

header() {
  echo -e "\n${BOLD}🧰 Devobox Installer${NC}\n"
}

usage() {
  echo "Usage: $0 [OPTIONS]"
  echo ""
  echo "Options:"
  echo "  -h, --help       Mostra esta ajuda"
  echo "  -y, --yes        Assume 'yes' para todas as perguntas"
  echo "  -n, --dry-run    Mostra o que seria feito sem executar"
  echo ""
}

# Parse args
ASSUME_YES=false
DRY_RUN=false
while [[ $# -gt 0 ]]; do
  case $1 in
    -h|--help) usage; exit 0 ;;
    -y|--yes) ASSUME_YES=true; shift ;;
    -n|--dry-run) DRY_RUN=true; shift ;;
    *) echo "Unknown option: $1"; usage; exit 1 ;;
  esac
done

run() {
  if $DRY_RUN; then
    echo -e "${YELLOW}[DRY-RUN]${NC} $*"
  else
    "$@"
  fi
}

check_distrobox() {
  header
  log_info "Verificando pré-requisitos..."

  if ! command -v distrobox &> /dev/null; then
    log_error "Distrobox não encontrado!"
    echo ""
    echo "Instale primeiro: https://distrobox.it"
    exit 1
  fi

  log_success "Distrobox encontrado: $(distrobox --version)"
}

check_existing() {
  log_info "Verificando instalação existente..."

  local needs_setup=false

  if [[ -f "$HOME/.config/distrobox/distrobox.ini" ]]; then
    log_warn "Configuração já existe em ~/.config/distrobox/distrobox.ini"
  else
    needs_setup=true
  fi

  if [[ -d "$DEVOBIN_DIR" && -f "$DEVOBIN_SCRIPT" ]]; then
    log_warn "Atalho 'devobox' já existe em ~/.local/bin/"
  else
    needs_setup=true
  fi

  if distrobox list 2>/dev/null | grep -q "devobox"; then
    log_warn "Container 'devobox' já existe"
  else
    needs_setup=true
  fi

  if $needs_setup && ! $ASSUME_YES; then
    echo ""
    read -rp "Deseja continuar com a instalação? [Y/n] " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]] && [[ -n $REPLY ]]; then
      log_info "Instalação cancelada."
      exit 0
    fi
  fi
}

setup_config() {
  log_info "Baixando configuração..."

  run mkdir -p "$HOME/.config/distrobox"
  run curl -fLo "$HOME/.config/distrobox/distrobox.ini" "$DISTROBOX_INI_URL"

  log_success "Configuração baixada em ~/.config/distrobox/distrobox.ini"
}

setup_home() {
  log_info "Criando diretório home do container..."

  run mkdir -p "$DEVOBIN_DIR"
  run mkdir -p "$DEVOBIN_DIR"

  if [[ ! -d "$DEVOBOX_DIR" ]]; then
    run mkdir -p "$DEVOBOX_DIR"
    log_success "Diretório criado: $DEVOBOX_DIR"
  else
    log_success "Diretório já existe: $DEVOBOX_DIR"
  fi
}

setup_shortcut() {
  log_info "Criando atalho 'devobox'..."

  run mkdir -p "$DEVOBIN_DIR"
  cat > "$DEVOBIN_SCRIPT" << 'EOF'
#!/usr/bin/env bash
distrobox enter -nw devobox
EOF
  run chmod +x "$DEVOBIN_SCRIPT"

  log_success "Atalho criado: $DEVOBIN_SCRIPT"
}

create_container() {
  echo ""
  log_info "Criando container..."

  if distrobox list 2>/dev/null | grep -q "devobox"; then
    log_warn "Container 'devobox' já existe. Pulando criação."
    return
  fi

  run distrobox assemble create --file "$HOME/.config/distrobox/distrobox.ini"

  echo ""
  log_success "Container criado com sucesso!"
}

finish() {
  echo ""
  echo -e "${BOLD}${GREEN}════════════════════════════════════════${NC}"
  echo -e "${BOLD}${GREEN}  Instalação concluída!${NC}"
  echo -e "${BOLD}${GREEN}════════════════════════════════════════${NC}"
  echo ""
  echo -e "  Entre no Devobox com:"
  echo -e "  ${BOLD}devobox${NC}  ou  ${BOLD}distrobox enter -nw devobox${NC}"
  echo ""
}

main() {
  check_distrobox
  check_existing
  setup_config
  setup_home
  setup_shortcut
  create_container
  finish
}

main
