#!/bin/sh
# Install the Puddi toolchain (`pud`) from GitHub releases.
#
#   curl -fsSL https://raw.githubusercontent.com/puddi-lang/puddi/main/install.sh | sh
#
# Environment:
#   PUD_VERSION          release tag to install, such as v0.1.0 (default: latest)
#   PUD_INSTALL          install root; pud goes to $PUD_INSTALL/bin (default: ~/.pud)
#   PUD_NO_MODIFY_PATH   set to 1 to leave shell profiles alone

set -eu

REPO="puddi-lang/puddi"
VERSION="${PUD_VERSION:-latest}"
ROOT="${PUD_INSTALL:-$HOME/.pud}"
BIN="$ROOT/bin"

say() { printf 'pud-install: %s\n' "$*"; }
die() { printf 'pud-install: error: %s\n' "$*" >&2; exit 1; }
need() { command -v "$1" > /dev/null 2>&1 || die "this script needs \`$1\`"; }

need uname
need tar
need mkdir

case "$(uname -s)" in
    Linux) os=unknown-linux-gnu ;;
    Darwin) os=apple-darwin ;;
    MINGW* | MSYS* | CYGWIN*) die "on Windows, run in PowerShell: irm https://raw.githubusercontent.com/$REPO/main/install.ps1 | iex" ;;
    *) die "unsupported OS: $(uname -s)" ;;
esac

case "$(uname -m)" in
    x86_64 | amd64) arch=x86_64 ;;
    aarch64 | arm64) arch=aarch64 ;;
    *) die "unsupported CPU architecture: $(uname -m)" ;;
esac

# A shell running under Rosetta 2 reports x86_64 on Apple silicon.
if [ "$os" = apple-darwin ] && [ "$arch" = x86_64 ] && [ "$(sysctl -n sysctl.proc_translated 2> /dev/null || echo 0)" = 1 ]; then
    arch=aarch64
fi

target="$arch-$os"
asset="pud-$target.tar.gz"
if [ "$VERSION" = latest ]; then
    base="https://github.com/$REPO/releases/latest/download"
else
    base="https://github.com/$REPO/releases/download/$VERSION"
fi

if command -v curl > /dev/null 2>&1; then
    fetch() { curl --proto '=https' --tlsv1.2 -fsSL "$1" -o "$2"; }
elif command -v wget > /dev/null 2>&1; then
    fetch() { wget -q --https-only "$1" -O "$2"; }
else
    die "this script needs \`curl\` or \`wget\`"
fi

tmp="$(mktemp -d 2> /dev/null || mktemp -d -t pud)"
trap 'rm -rf "$tmp"' EXIT INT TERM

say "downloading $asset ($VERSION)"
fetch "$base/$asset" "$tmp/$asset" || die "download failed: $base/$asset"
fetch "$base/$asset.sha256" "$tmp/$asset.sha256" || die "download failed: $base/$asset.sha256"

expected="$(cut -d ' ' -f 1 < "$tmp/$asset.sha256")"
if command -v sha256sum > /dev/null 2>&1; then
    actual="$(sha256sum "$tmp/$asset" | cut -d ' ' -f 1)"
elif command -v shasum > /dev/null 2>&1; then
    actual="$(shasum -a 256 "$tmp/$asset" | cut -d ' ' -f 1)"
else
    die "this script needs \`sha256sum\` or \`shasum\` to verify the download"
fi
[ "$expected" = "$actual" ] || die "checksum mismatch for $asset (expected $expected, got $actual)"

tar -xzf "$tmp/$asset" -C "$tmp"
mkdir -p "$BIN"
cp "$tmp/pud-$target/pud" "$BIN/pud.tmp"
chmod 755 "$BIN/pud.tmp"
mv -f "$BIN/pud.tmp" "$BIN/pud"
say "installed $("$BIN/pud" version) to $BIN/pud"

case ":$PATH:" in
    *":$BIN:"*) on_path=1 ;;
    *) on_path=0 ;;
esac

if [ "$on_path" = 0 ] && [ "${PUD_NO_MODIFY_PATH:-0}" != 1 ]; then
    line="export PATH=\"$BIN:\$PATH\""
    updated=""
    for rc in "$HOME/.profile" "$HOME/.bashrc" "$HOME/.zshrc"; do
        case "$rc" in
            */.profile) ;;
            *) [ -f "$rc" ] || continue ;;
        esac
        if ! grep -qsF "$line" "$rc"; then
            printf '\n# Puddi toolchain\n%s\n' "$line" >> "$rc"
            updated="$updated $rc"
        fi
    done
    if command -v fish > /dev/null 2>&1; then
        fish_conf="$HOME/.config/fish/conf.d/pud.fish"
        if [ ! -f "$fish_conf" ]; then
            mkdir -p "$(dirname "$fish_conf")"
            printf 'fish_add_path -g %s\n' "$BIN" > "$fish_conf"
            updated="$updated $fish_conf"
        fi
    fi
    [ -n "$updated" ] && say "added $BIN to PATH in:$updated"
    say "open a new shell, or run: export PATH=\"$BIN:\$PATH\""
fi

if command -v node > /dev/null 2>&1; then
    major="$(node -p 'process.versions.node.split(".")[0]' 2> /dev/null || echo 0)"
    [ "$major" -ge 22 ] 2> /dev/null || say "note: \`pud test\` and the browser target need Node.js 22 or newer (found $(node --version))"
else
    say "note: \`pud test\` and the browser target need Node.js 22 or newer; \`pud run\` works without it"
fi

say "get started: pud new hello && cd hello && pud run"
