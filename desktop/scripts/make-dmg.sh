#!/usr/bin/env bash
# Build a distributable DMG for the built AI Task Notify.app.
#
# `pnpm tauri build` produces the .app bundle; this script wraps it into a DMG
# using plain `hdiutil` (the create-dmg flow used by `tauri build --bundles dmg`
# is flaky on some machines, so locally the DMG is built separately).
#
# 注意：仓库是 Cargo workspace，bundle 产物在仓库根目录的 target/ 下。
#
# Usage: bash desktop/scripts/make-dmg.sh   (或根目录 pnpm dmg)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BUNDLE_ROOT="$ROOT/target/release/bundle"

VERSION="$(sed -n 's/.*"version": *"\([^"]*\)".*/\1/p' "$ROOT/desktop/src-tauri/tauri.conf.json" | head -1)"
ARCH="$(uname -m)"
case "$ARCH" in
  arm64)  ARCH="aarch64" ;;
  x86_64) ARCH="x64" ;;
esac

APP="$BUNDLE_ROOT/macos/AI Task Notify.app"
OUT="$BUNDLE_ROOT/dmg/AI Task Notify_${VERSION}_${ARCH}.dmg"

if [[ ! -d "$APP" ]]; then
  echo "❌ 未找到 $APP，请先运行：pnpm build:app" >&2
  exit 1
fi

mkdir -p "$(dirname "$OUT")"
rm -f "$OUT"
hdiutil create -volname "AI Task Notify" -srcfolder "$APP" -ov -format UDZO "$OUT"
echo "✅ DMG 已生成：$OUT"
