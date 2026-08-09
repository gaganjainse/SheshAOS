#!/usr/bin/env bash
set -euo pipefail

MODE="${1:---full}"

log() { printf '\n==> %s\n' "$*"; }
warn() { printf '\n==> %s\n' "$*" >&2; }
have() { command -v "$1" >/dev/null 2>&1; }
run() { log "$*"; bash -lc "$*"; }

need_sudo() {
  sudo -v
}

ensure_line() {
  local file="$1"
  local line="$2"
  touch "$file"
  grep -qxF "$line" "$file" || printf '%s\n' "$line" >> "$file"
}

apt_install() {
  need_sudo
  run "sudo apt update"
  run "sudo apt install -y $*"
}

setup_workspace() {
  log "Creating Workspace layout"
  mkdir -p "$HOME/Workspace"/{
    AI/{Models/{Ollama,GGUF,HuggingFace,Embeddings,Diffusion},Datasets,FineTuning,Benchmarks,Experiments,Training,Logs,Cache},
    Projects/{Active,Archive,Playground,Templates},
    Learning/{Python,Rust,Linux,AI},
    Scripts,
    Containers,
    Notes,
    Downloads,
    Assets,
    Backups,
    Temp
  }
}

setup_shell() {
  log "Configuring shell helpers"
  ensure_line "$HOME/.bashrc" 'export PNPM_HOME="$HOME/.local/share/pnpm"'
  ensure_line "$HOME/.bashrc" 'case ":$PATH:" in'
  ensure_line "$HOME/.bashrc" '  *":$PNPM_HOME/bin:"*) ;;'
  ensure_line "$HOME/.bashrc" '  *) export PATH="$PNPM_HOME/bin:$PATH" ;;'
  ensure_line "$HOME/.bashrc" 'esac'
  ensure_line "$HOME/.bashrc" 'export PATH="$HOME/.cargo/bin:$PATH"'
  ensure_line "$HOME/.bashrc" 'eval "$(zoxide init bash)"'
  ensure_line "$HOME/.bashrc" 'eval "$(starship init bash)"'
  ensure_line "$HOME/.bashrc" 'alias ll="eza -lah --icons"'
  ensure_line "$HOME/.bashrc" 'alias ls="eza --icons"'
  ensure_line "$HOME/.bashrc" 'alias cat="bat"'
}

setup_git() {
  log "Configuring Git defaults"
  git config --global init.defaultBranch main || true
  git config --global pull.rebase false || true
  git config --global fetch.prune true || true
}

configure_pnpm() {
  if have pnpm; then
    log "Configuring pnpm"
    pnpm setup || true
  fi
}

install_common_tools() {
  apt_install     git gh curl wget ca-certificates build-essential pkg-config cmake ninja-build     unzip zip xz-utils ripgrep fd-find jq tree htop btop tmux     python3 python3-pip python3-venv python3-dev python-is-python3     git-lfs ffmpeg flatpak topgrade libssl-dev

  if have fdfind && ! have fd; then
    ensure_line "$HOME/.bashrc" 'alias fd=fdfind'
  fi
}

install_rust_tools() {
  if ! have cargo; then
    warn "cargo not found; skipping Rust tools"
    return
  fi

  if ! cargo install --list 2>/dev/null | grep -q '^cargo-update '; then
    cargo install cargo-update || true
  fi

  for tool in cargo-cache cargo-audit cargo-deny; do
    if ! cargo install --list 2>/dev/null | grep -q "^${tool} "; then
      cargo install "$tool" || true
    fi
  done
}

maybe_install_ollama() {
  if have ollama; then
    log "Ollama already installed"
  else
    warn "Ollama not found; install manually if desired:"
    echo "curl -fsSL https://ollama.com/install.sh | sh"
  fi
}

refresh_system() {
  run "sudo apt update"
  run "sudo apt full-upgrade -y"
  run "sudo apt autoremove -y"
  have snap && run "sudo snap refresh" || true
  have flatpak && run "flatpak update -y" || true
  have fwupdmgr && run "fwupdmgr refresh || true" && run "fwupdmgr get-updates || true" || true
}

run_topgrade() {
  if have topgrade; then
    topgrade -y || true
  else
    warn "topgrade not installed"
  fi
}

main() {
  case "$MODE" in
    --install)
      install_common_tools
      setup_workspace
      setup_shell
      setup_git
      configure_pnpm
      install_rust_tools
      maybe_install_ollama
      ;;
    --update)
      refresh_system
      run_topgrade
      ;;
    --full)
      install_common_tools
      setup_workspace
      setup_shell
      setup_git
      configure_pnpm
      install_rust_tools
      maybe_install_ollama
      refresh_system
      run_topgrade
      ;;
    *)
      echo "Usage: $0 [--install|--update|--full]"
      exit 1
      ;;
  esac

  log "Done"
}

main "$@"
