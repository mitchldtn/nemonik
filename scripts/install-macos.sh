#!/bin/sh
# Installs the nemonik CLI on macOS from GitHub Releases, without Homebrew.
#
#   curl -fsSL https://raw.githubusercontent.com/mitchldtn/nemonik/main/scripts/install-macos.sh | sh
#
# To upgrade, re-run it.
# Env: NEMONIK_VERSION (default latest), NEMONIK_INSTALL_DIR (default ~/.local/bin).
set -eu

REPO="mitchldtn/nemonik"
INSTALL_DIR="${NEMONIK_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${NEMONIK_VERSION:-}"
VERSION="${VERSION#v}"

fail() {
  echo "error: $*" >&2
  exit 1
}

[ "$(uname -s)" = Darwin ] ||
  fail "this installer is for macOS; on Linux use: curl -fsSL https://raw.githubusercontent.com/$REPO/main/scripts/install-linux.sh | sh"

for tool in curl shasum ditto install mktemp; do
  command -v "$tool" >/dev/null 2>&1 || fail "$tool is required"
done

if [ -z "$VERSION" ]; then
  latest=$(curl -fsSLI -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest") ||
    fail "could not reach github.com/$REPO/releases"
  VERSION=${latest##*/v}
  [ -n "$VERSION" ] && [ "$VERSION" != "$latest" ] || fail "could not determine the latest release"
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
trap 'exit 130' INT TERM

url="https://github.com/$REPO/releases/download/v$VERSION/nemonik.zip"
echo "==> Downloading nemonik v$VERSION (universal)"
curl -fsSL "$url" -o "$tmp/nemonik.zip" || fail "could not download $url"
curl -fsSL "$url.sha256" -o "$tmp/nemonik.zip.sha256" || fail "could not download $url.sha256"
(cd "$tmp" && shasum -a 256 -c nemonik.zip.sha256 >/dev/null) || fail "checksum mismatch for nemonik.zip"
ditto -x -k "$tmp/nemonik.zip" "$tmp/out"
[ -f "$tmp/out/dist/nemonik" ] || fail "nemonik.zip has no dist/nemonik"
mkdir -p "$INSTALL_DIR"
install -m 755 "$tmp/out/dist/nemonik" "$INSTALL_DIR/nemonik"

nk="$INSTALL_DIR/nk"
if [ ! -e "$nk" ] && [ ! -L "$nk" ]; then
  ln -s nemonik "$nk"
elif [ "$(readlink "$nk" 2>/dev/null)" != nemonik ]; then
  echo "Note: $nk already exists and is not nemonik's alias; left it alone."
fi

echo "==> Installed to $INSTALL_DIR:"
"$INSTALL_DIR/nemonik" --version

case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *) echo "Note: $INSTALL_DIR is not on your PATH. Add it to ~/.zprofile, e.g.: export PATH=\"$INSTALL_DIR:\$PATH\"" ;;
esac

if ! command -v tmux >/dev/null 2>&1; then
  echo
  echo "Note: nemonik needs tmux, which is not installed. Install it:"
  echo "  Homebrew:  brew install tmux"
  echo "  MacPorts:  sudo port install tmux"
fi
