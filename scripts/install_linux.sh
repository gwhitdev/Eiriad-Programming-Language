#!/usr/bin/env bash
set -euo pipefail

METHOD="git"
REPO_URL="https://github.com/gwhitdev/Eiriad-Programming-Language.git"

usage() {
  cat <<'EOF'
Install Eiriad on Linux.

Usage:
  install_linux.sh [--method git|source] [--repo-url URL]

Options:
  --method   Install method:
             - git (default): cargo install from GitHub
             - source: cargo install from local repository checkout
  --repo-url Repository URL for --method git
  -h, --help Show this help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --method)
      METHOD="${2:-}"
      shift 2
      ;;
    --repo-url)
      REPO_URL="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "This installer is intended for Linux only." >&2
  exit 1
fi

ensure_cargo() {
  if command -v cargo >/dev/null 2>&1; then
    return
  fi

  echo "cargo was not found. Installing Rust toolchain with rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

  # shellcheck disable=SC1090
  source "$HOME/.cargo/env"

  if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo is still not available after rustup install." >&2
    exit 1
  fi
}

ensure_cargo

install_from_git() {
  echo "Installing eiriad from $REPO_URL ..."
  cargo install --git "$REPO_URL" --bin eiriad --locked --force
}

install_from_source() {
  local repo_root
  repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

  if [[ ! -f "$repo_root/Cargo.toml" ]]; then
    echo "Could not locate Cargo.toml at $repo_root" >&2
    exit 1
  fi

  echo "Installing eiriad from local source at $repo_root ..."
  cargo install --path "$repo_root" --bin eiriad --locked --force
}

case "$METHOD" in
  git)
    install_from_git
    ;;
  source)
    install_from_source
    ;;
  *)
    echo "Invalid --method '$METHOD'. Expected 'git' or 'source'." >&2
    exit 1
    ;;
esac

if command -v eiriad >/dev/null 2>&1; then
  echo "Installed: $(command -v eiriad)"
else
  echo "Install completed, but 'eiriad' is not in PATH yet." >&2
  echo "Add ~/.cargo/bin to PATH, or run: source ~/.cargo/env" >&2
fi

eiriad --help >/dev/null 2>&1 || true

echo "Done. Run 'eiriad' to start the REPL."
