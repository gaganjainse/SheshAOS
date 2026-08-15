#!/usr/bin/env bash
# shesh-bootstrap.sh — provision the Shesh host machine layout & tools.
#
#   --install   tools + workspace + shell + git defaults
#   --update    system refresh only
#   --full      install + update (default)
#
# Idempotent: every step is safe to re-run (re-runs verify state instead of
# redoing work — no silent `|| true` anywhere: a failure aborts the script
# loudly via `set -euo pipefail`).
set -euo pipefail

MODE="${1:---full}"

log() { printf '\n==> %s\n' "$*"; }
warn() { printf '\n==> %s\n' "$*" >&2; }
have() { command -v "$1" >/dev/null 2>&1; }
run() {
    log "$*"
    bash -lc "$*"
}

need_sudo() {
    sudo -v
}

ensure_line() {
    local file="$1"
    local line="$2"
    touch "$file"
    grep -qxF "$line" "$file" || printf '%s\n' "$line" >>"$file"
}

apt_install() {
    need_sudo
    run "sudo apt update"
    run "sudo apt install -y $*"
}

WORKSPACE_DIRS=(
    AI/Models/Ollama
    AI/Models/GGUF
    AI/Models/HuggingFace
    AI/Models/Embeddings
    AI/Models/Diffusion
    AI/Datasets
    AI/FineTuning
    AI/Benchmarks
    AI/Experiments
    AI/Training
    AI/Logs
    AI/Cache
    Projects/Active
    Projects/Archive
    Projects/Playground
    Projects/Templates
    Learning/Python
    Learning/Rust
    Learning/Linux
    Learning/AI
    Scripts
    Containers
    Notes
    Downloads
    Assets
    Backups
    Temp
)

setup_workspace() {
    log "Creating Workspace layout"
    local d
    for d in "${WORKSPACE_DIRS[@]}"; do
        mkdir -p "$HOME/Workspace/$d"
    done
}

# shellcheck disable=SC2016
# The single-quoted lines below are WRITTEN VERBATIM into ~/.bashrc; the $
# must not expand in this process. Scoped to the function that follows.
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
    git config --global init.defaultBranch main
    git config --global pull.rebase false
    git config --global fetch.prune true
}

configure_pnpm() {
    if have pnpm; then
        log "Configuring pnpm"
        if [ ! -d "${PNPM_HOME:-$HOME/.local/share/pnpm}" ]; then
            pnpm setup
        else
            log "pnpm home already configured — skipping"
        fi
    else
        warn "pnpm not found; skipping pnpm setup"
    fi
}

install_common_tools() {
    apt_install \
        git gh curl wget ca-certificates build-essential pkg-config cmake ninja-build \
        unzip zip xz-utils ripgrep fd-find jq tree htop btop tmux \
        python3 python3-pip python3-venv python3-dev python-is-python3 \
        git-lfs ffmpeg flatpak topgrade libssl-dev

    if have fdfind && ! have fd; then
        ensure_line "$HOME/.bashrc" 'alias fd=fdfind'
    fi
}

install_rust_tools() {
    if ! have cargo; then
        warn "cargo not found; skipping Rust tools"
        return
    fi

    local tool
    for tool in cargo-update cargo-cache cargo-audit cargo-deny; do
        if cargo install --list | grep -q "^${tool} "; then
            log "cargo tool present: $tool — skipping"
        else
            cargo install "$tool"
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
    if have snap; then run "sudo snap refresh"; fi
    if have flatpak; then run "flatpak update -y"; fi
    if have fwupdmgr; then
        run "fwupdmgr refresh"
        run "fwupdmgr get-updates"
    fi
}

run_topgrade() {
    if have topgrade; then
        topgrade -y
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
