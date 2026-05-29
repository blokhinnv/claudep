#!/usr/bin/env bash
set -euo pipefail

REPO="blokhinnv/claudep"
DEFAULT_UPSTREAM="socks5://127.0.0.1:1080"
INSTALL_DIR="${CLAUDEP_INSTALL_DIR:-$HOME/.local/bin}"
CLAUDEP_HOME="${CLAUDEP_HOME:-$HOME/.local/share/claudep}"
TEMPLATES_DIR="${CLAUDEP_TEMPLATES:-$CLAUDEP_HOME/templates}"

info() { printf '==> %s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }

detect_platform() {
  local os arch
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"
  case "$os" in
    darwin|linux) ;;
    *) echo "unsupported OS: $os (use macOS or Linux)" >&2; exit 1 ;;
  esac
  case "$arch" in
    arm64|aarch64) arch="arm64" ;;
    x86_64|amd64) arch="amd64" ;;
    *) echo "unsupported architecture: $arch" >&2; exit 1 ;;
  esac
  printf '%s-%s' "$os" "$arch"
}

download_release_asset() {
  local name="$1" dest="$2"
  local api="https://api.github.com/repos/${REPO}/releases/latest"
  local url

  if ! command -v curl >/dev/null 2>&1; then
    echo "curl is required" >&2
    exit 1
  fi

  url="$(curl -fsSL "$api" | sed -n 's/.*"browser_download_url": "\([^"]*\/'"${name}"'\)".*/\1/p' | head -n1)"
  if [[ -z "$url" ]]; then
    echo "could not find release asset: $name" >&2
    exit 1
  fi
  curl -fsSL "$url" -o "$dest"
}

prompt_upstream() {
  if [[ -n "${CLAUDEP_UPSTREAM:-}" ]]; then
    info "using CLAUDEP_UPSTREAM from environment"
    return
  fi

  local input
  if [[ -t 0 ]]; then
    read -r -p "CLAUDEP_UPSTREAM [${DEFAULT_UPSTREAM}]: " input
    CLAUDEP_UPSTREAM="${input:-$DEFAULT_UPSTREAM}"
  else
    CLAUDEP_UPSTREAM="$DEFAULT_UPSTREAM"
    info "non-interactive install; defaulting CLAUDEP_UPSTREAM=$CLAUDEP_UPSTREAM"
  fi
}

write_shell_snippet() {
  local profile marker block
  marker="# claudep"
  block="${marker}
export CLAUDEP_HOME=\"\${CLAUDEP_HOME:-\$HOME/.local/share/claudep}\"
export CLAUDEP_TEMPLATES=\"\${CLAUDEP_TEMPLATES:-\$CLAUDEP_HOME/templates}\"
export CLAUDEP_UPSTREAM=\"\${CLAUDEP_UPSTREAM:-${CLAUDEP_UPSTREAM}}\""

  if [[ -n "${CLAUDEP_INSTALL_DIR:-}" ]]; then
    block="${block}
export PATH=\"${INSTALL_DIR}:\$PATH\""
  else
    block="${block}
export PATH=\"\$HOME/.local/bin:\$PATH\""
  fi

  for profile in "$HOME/.zshrc" "$HOME/.bashrc"; do
    if [[ -f "$profile" ]] && grep -q "$marker" "$profile"; then
      info "shell snippet already present in $profile"
      continue
    fi
    if [[ -f "$profile" ]] || [[ "$profile" == "$HOME/.zshrc" ]]; then
      {
        echo ""
        echo "$block"
      } >>"$profile"
      info "updated $profile"
    fi
  done
}

main() {
  local platform tmpdir binary_name

  platform="$(detect_platform)"
  binary_name="claudep-${platform}"
  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' EXIT

  info "installing claudep for ${platform}"
  mkdir -p "$INSTALL_DIR" "$CLAUDEP_HOME" "$TEMPLATES_DIR"

  prompt_upstream

  info "downloading ${binary_name}"
  download_release_asset "$binary_name" "$tmpdir/claudep"
  chmod +x "$tmpdir/claudep"
  mv "$tmpdir/claudep" "$INSTALL_DIR/claudep"

  info "downloading templates"
  download_release_asset "templates.tar.gz" "$tmpdir/templates.tar.gz"
  tar -xzf "$tmpdir/templates.tar.gz" -C "$TEMPLATES_DIR"

  write_shell_snippet

  if ! command -v docker >/dev/null 2>&1; then
    warn "docker not found in PATH"
  elif ! docker compose version >/dev/null 2>&1; then
    warn "docker compose v2 not found"
  fi

  info "running claudep doctor"
  CLAUDEP_HOME="$CLAUDEP_HOME" CLAUDEP_TEMPLATES="$TEMPLATES_DIR" CLAUDEP_UPSTREAM="$CLAUDEP_UPSTREAM" \
    "$INSTALL_DIR/claudep" doctor || warn "doctor reported issues (see above)"

  info "installed claudep to $INSTALL_DIR/claudep"
  info "reload your shell: source ~/.zshrc  (or ~/.bashrc)"
}

main "$@"
