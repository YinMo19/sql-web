#!/bin/sh
set -eu

REPO="YinMo19/sql-web"
VERSION="${SQL_WEB_VERSION:-latest}"
INSTALL_DIR="${SQL_WEB_INSTALL_DIR:-$HOME/.local/bin}"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: required command '$1' not found" >&2
    exit 1
  fi
}

case "$(uname -s)" in
  Darwin) platform="apple-darwin" ;;
  Linux) platform="unknown-linux-gnu" ;;
  *)
    echo "error: unsupported OS: $(uname -s)" >&2
    exit 1
    ;;
esac

case "$(uname -m)" in
  x86_64|amd64) arch="x86_64" ;;
  arm64|aarch64) arch="aarch64" ;;
  *)
    echo "error: unsupported architecture: $(uname -m)" >&2
    exit 1
    ;;
esac

target="${arch}-${platform}"
asset="sql-web-${target}.tar.gz"
if [ "$VERSION" = "latest" ]; then
  url="https://github.com/${REPO}/releases/latest/download/${asset}"
else
  url="https://github.com/${REPO}/releases/download/${VERSION}/${asset}"
fi

tmpdir="$(mktemp -d)"
cleanup() {
  rm -rf "$tmpdir"
}
trap cleanup EXIT INT TERM

archive="$tmpdir/$asset"
if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$url" -o "$archive"
elif command -v wget >/dev/null 2>&1; then
  wget -q "$url" -O "$archive"
else
  echo "error: curl or wget is required" >&2
  exit 1
fi

need_cmd tar
mkdir -p "$INSTALL_DIR"
tar -xzf "$archive" -C "$tmpdir"
install -m 755 "$tmpdir/sql-web-${target}/sql-web" "$INSTALL_DIR/sql-web"

echo "sql-web installed to $INSTALL_DIR/sql-web"
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    echo "note: $INSTALL_DIR is not in PATH. Add it to your shell profile, for example:"
    echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
    ;;
esac
