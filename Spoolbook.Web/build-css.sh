#!/usr/bin/env bash
# Rebuilds wwwroot/css/tailwind.css from Styles/tailwind.css + Tailwind class usage
# across Components/**/*.razor. Run after adding/changing Tailwind classes in markup.
set -euo pipefail
cd "$(dirname "$0")"

BIN=.tailwind/tailwindcss
if [ ! -x "$BIN" ]; then
    mkdir -p .tailwind
    os=$(uname -s)
    arch=$(uname -m)
    case "$os-$arch" in
        Darwin-arm64) target=tailwindcss-macos-arm64 ;;
        Darwin-x86_64) target=tailwindcss-macos-x64 ;;
        Linux-x86_64) target=tailwindcss-linux-x64 ;;
        Linux-aarch64) target=tailwindcss-linux-arm64 ;;
        *) echo "Unsupported platform: $os-$arch — download manually from https://github.com/tailwindlabs/tailwindcss/releases" >&2; exit 1 ;;
    esac
    curl -sL -o "$BIN" "https://github.com/tailwindlabs/tailwindcss/releases/latest/download/$target"
    chmod +x "$BIN"
fi

"$BIN" -i Styles/tailwind.css -o wwwroot/css/tailwind.css --minify
