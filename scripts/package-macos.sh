#!/bin/bash
set -e

# SSH Tunnel Manager - macOS Application Packaging Script
# Usage: ./scripts/package-macos.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
APP_NAME="SSH Tunnel Manager"
BUNDLE_ID="com.cooper.ssh-tunnel-manager"
VERSION=$(grep '^version' "$PROJECT_DIR/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')

echo "🔨 Building SSH Tunnel Manager v$VERSION..."
echo ""

# Build release binary with GUI feature
cd "$PROJECT_DIR"
cargo build --release --features gui

# Create app bundle directories
APP_DIR="$PROJECT_DIR/target/release/$APP_NAME.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"

rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR"
mkdir -p "$RESOURCES_DIR"

# Copy binary
cp "$PROJECT_DIR/target/release/ssh-tunnel-manager" "$MACOS_DIR/"

# Create launch script (to pass --gui argument)
cat > "$MACOS_DIR/launch.sh" << 'LAUNCHEOF'
#!/bin/bash
DIR="$(cd "$(dirname "$0")" && pwd)"
exec "$DIR/ssh-tunnel-manager" --gui "$@"
LAUNCHEOF
chmod +x "$MACOS_DIR/launch.sh"

# Create Info.plist
cat > "$CONTENTS_DIR/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>$APP_NAME</string>
    <key>CFBundleDisplayName</key>
    <string>$APP_NAME</string>
    <key>CFBundleIdentifier</key>
    <string>$BUNDLE_ID</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>launch.sh</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>LSMinimumSystemVersion</key>
    <string>12.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.developer-tools</string>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright © 2024 Cooper. All rights reserved.</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
</dict>
</plist>
EOF

# Create app icon
ICONSET_DIR="$RESOURCES_DIR/AppIcon.iconset"
mkdir -p "$ICONSET_DIR"

# Create SVG icon
SVG_FILE="/tmp/ssh_tunnel_icon.svg"
cat > "$SVG_FILE" << 'SVGEOF'
<?xml version="1.0" encoding="UTF-8"?>
<svg width="1024" height="1024" viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <linearGradient id="bg" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" style="stop-color:#4F46E5"/>
      <stop offset="100%" style="stop-color:#7C3AED"/>
    </linearGradient>
  </defs>
  <rect width="1024" height="1024" rx="180" fill="url(#bg)"/>
  <g transform="translate(512, 512)">
    <rect x="-300" y="-250" width="600" height="450" rx="30" fill="#1E1B4B" stroke="#A5B4FC" stroke-width="8"/>
    <rect x="-300" y="-250" width="600" height="60" rx="30" fill="#312E81"/>
    <rect x="-300" y="-210" width="600" height="20" fill="#312E81"/>
    <circle cx="-250" cy="-220" r="15" fill="#EF4444"/>
    <circle cx="-200" cy="-220" r="15" fill="#F59E0B"/>
    <circle cx="-150" cy="-220" r="15" fill="#22C55E"/>
    <text x="0" y="-50" text-anchor="middle" font-family="monospace" font-size="120" font-weight="bold" fill="#A5B4FC">SSH</text>
    <path d="M-180 80 L180 80" stroke="#22C55E" stroke-width="16" stroke-linecap="round"/>
    <path d="M140 50 L180 80 L140 110" stroke="#22C55E" stroke-width="16" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
    <path d="M-140 50 L-180 80 L-140 110" stroke="#22C55E" stroke-width="16" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
  </g>
</svg>
SVGEOF

# Generate icon sizes
if command -v rsvg-convert &> /dev/null; then
    for size in 16 32 64 128 256 512 1024; do
        rsvg-convert -w $size -h $size "$SVG_FILE" > "$ICONSET_DIR/icon_${size}x${size}.png"
        if [ $size -le 512 ]; then
            rsvg-convert -w $((size*2)) -h $((size*2)) "$SVG_FILE" > "$ICONSET_DIR/icon_${size}x${size}@2x.png"
        fi
    done
else
    echo "⚠️  rsvg-convert not found. Install librsvg for icon generation."
    echo "   brew install librsvg"
    exit 1
fi

# Convert to icns
iconutil -c icns "$ICONSET_DIR" -o "$RESOURCES_DIR/AppIcon.icns"
rm -rf "$ICONSET_DIR"
rm -f "$SVG_FILE"

echo ""
echo "✅ Application packaged successfully!"
echo ""
echo "📦 Location: $APP_DIR"
echo "📊 Size: $(du -sh "$APP_DIR" | cut -f1)"
echo ""
echo "To run the application:"
echo "  open \"$APP_DIR\""
echo ""
echo "To copy to Applications:"
echo "  cp -r \"$APP_DIR\" /Applications/"
