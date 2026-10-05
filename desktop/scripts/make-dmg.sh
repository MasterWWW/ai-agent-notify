#!/usr/bin/env bash
# Build a distributable DMG for the built AI Task Notify.app.
#
# `pnpm tauri build` produces the .app bundle; this script wraps it into a DMG
# using plain `hdiutil` (the create-dmg flow used by `tauri build --target dmg`
# is flaky on some machines, so DMG is built separately).
#
# Usage: bash scripts/make-dmg.sh
set -euo pipefail

cd "$(dirname "$0")/.."

APP="src-tauri/target/release/bundle/macos/AI Task Notify.app"
OUT="src-tauri/target/release/bundle/dmg/AI Task Notify_0.1.0_x64.dmg"

if [[ ! -d "$APP" ]]; then
  echo "❌ 未找到 $APP，请先运行：pnpm tauri build" >&2
  exit 1
fi

mkdir -p "$(dirname "$OUT")"
rm -f "$OUT"
hdiutil create -volname "AI Task Notify" -srcfolder "$APP" -ov -format UDZO "$OUT"
echo "✅ DMG 已生成：$OUT"
