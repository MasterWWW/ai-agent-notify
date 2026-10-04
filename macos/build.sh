#!/bin/bash
# Build the AI Task Notify macOS menu bar app.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP_NAME="AI Task Notify"
APP_DIR="$ROOT/macos/build/$APP_NAME.app"

rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources/server"

cat > "$APP_DIR/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>AI Task Notify</string>
  <key>CFBundleIdentifier</key><string>local.ai-task-notify.macos</string>
  <key>CFBundleName</key><string>AI Task Notify</string>
  <key>CFBundleDisplayName</key><string>AI Task Notify</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

swiftc -parse-as-library -O -o "$APP_DIR/Contents/MacOS/$APP_NAME" \
  "$ROOT/macos/AITaskNotifyApp.swift" \
  -framework AppKit -framework SwiftUI

# Bundle a Node runtime so the app works without depending on PATH (self-contained).
NODE=""
for p in "$HOME"/.nvmd/versions/*/bin/node; do [ -x "$p" ] && NODE="$p"; done
if [ -z "$NODE" ]; then
  for p in /opt/homebrew/bin/node /usr/local/bin/node; do [ -x "$p" ] && NODE="$p" && break; done
fi
if [ -n "$NODE" ]; then
  cp "$NODE" "$APP_DIR/Contents/Resources/node"
  chmod +x "$APP_DIR/Contents/Resources/node"
  echo "Bundled node: $NODE"
else
  echo "WARN: node not found, app will try to resolve at runtime"
fi

cp "$ROOT/server/dist/bundle.cjs" "$APP_DIR/Contents/Resources/server/bundle.cjs"

echo "Built: $APP_DIR"
