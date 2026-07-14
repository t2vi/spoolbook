#!/usr/bin/env bash
# Builds Spoolbook.app — a real macOS app bundle with a custom Dock icon.
# `dotnet run` never produces one (see docs/adr and README): macOS reads the
# Dock icon from Contents/Info.plist + Contents/Resources/*.icns, which only
# exist inside an .app bundle, not a bare published executable.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RID="${1:-osx-arm64}"
APP_NAME="Spoolbook"
DIST_DIR="$REPO_ROOT/dist"
APP_DIR="$DIST_DIR/$APP_NAME.app"
PUBLISH_DIR="$REPO_ROOT/Spoolbook.Desktop/bin/Release/net10.0/$RID/publish"

echo "Publishing (RID=$RID)..."
dotnet publish "$REPO_ROOT/Spoolbook.Desktop" -c Release -r "$RID" --self-contained true -p:PublishSingleFile=true -o "$PUBLISH_DIR"

rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources"

cp -R "$PUBLISH_DIR"/. "$APP_DIR/Contents/MacOS/"
cp "$REPO_ROOT/Spoolbook.Desktop/Assets/spoolbook.icns" "$APP_DIR/Contents/Resources/spoolbook.icns"
chmod +x "$APP_DIR/Contents/MacOS/Spoolbook.Desktop"

cat > "$APP_DIR/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Spoolbook</string>
    <key>CFBundleDisplayName</key>
    <string>Spoolbook</string>
    <key>CFBundleIdentifier</key>
    <string>com.spoolbook.desktop</string>
    <key>CFBundleExecutable</key>
    <string>Spoolbook.Desktop</string>
    <key>CFBundleIconFile</key>
    <string>spoolbook</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST

echo "Built $APP_DIR"
echo "Run: open \"$APP_DIR\""
