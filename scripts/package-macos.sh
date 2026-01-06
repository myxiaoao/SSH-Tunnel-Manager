#!/bin/bash
set -e

# SSH Tunnel Manager - macOS Application Packaging Script
# Usage: ./scripts/package-macos.sh [--skip-build] [--universal]

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Extract metadata from Cargo.toml
APP_NAME="SSH Tunnel Manager"
PACKAGE_NAME=$(grep '^name' "$PROJECT_DIR/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
VERSION=$(grep '^version' "$PROJECT_DIR/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
BUNDLE_ID="com.myxiaoao.${PACKAGE_NAME}"

# Parse arguments
SKIP_BUILD=false
UNIVERSAL=false
for arg in "$@"; do
    case $arg in
        --skip-build) SKIP_BUILD=true ;;
        --universal) UNIVERSAL=true ;;
    esac
done

echo "🔨 Packaging $APP_NAME v$VERSION"
echo "   Bundle ID: $BUNDLE_ID"
echo ""

cd "$PROJECT_DIR"

# Build release binary with GUI feature
if [ "$SKIP_BUILD" = false ]; then
    if [ "$UNIVERSAL" = true ]; then
        echo "📦 Building universal binary (arm64 + x86_64)..."
        # Ensure both targets are installed
        rustup target add aarch64-apple-darwin x86_64-apple-darwin 2>/dev/null || true

        cargo build --release --features gui --target aarch64-apple-darwin
        cargo build --release --features gui --target x86_64-apple-darwin

        # Create universal binary
        mkdir -p target/release
        lipo -create \
            target/aarch64-apple-darwin/release/"$PACKAGE_NAME" \
            target/x86_64-apple-darwin/release/"$PACKAGE_NAME" \
            -output target/release/"$PACKAGE_NAME"
    else
        echo "📦 Building release binary..."
        cargo build --release --features gui
    fi
fi

# Verify binary exists
BINARY_PATH="$PROJECT_DIR/target/release/$PACKAGE_NAME"
if [ ! -f "$BINARY_PATH" ]; then
    echo "❌ Binary not found: $BINARY_PATH"
    exit 1
fi

# Create app bundle directories
APP_DIR="$PROJECT_DIR/target/release/$APP_NAME.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"

echo "📁 Creating app bundle..."
rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"

# Copy binary
cp "$BINARY_PATH" "$MACOS_DIR/"

# Create launcher script
cat > "$MACOS_DIR/launcher" << 'EOF'
#!/bin/bash
DIR="$(cd "$(dirname "$0")" && pwd)"
exec "$DIR/ssh-tunnel-manager" --gui "$@"
EOF
chmod +x "$MACOS_DIR/launcher"

# Create Info.plist
cat > "$CONTENTS_DIR/Info.plist" << PLIST
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
    <string>launcher</string>
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
    <string>Copyright © 2026 Cooper. MIT License.</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
</dict>
</plist>
PLIST

# Generate app icon
echo "🎨 Generating app icon..."
generate_icon() {
    ICONSET_DIR="$RESOURCES_DIR/AppIcon.iconset"
    mkdir -p "$ICONSET_DIR"

    # SVG icon definition
    SVG_CONTENT='<?xml version="1.0" encoding="UTF-8"?>
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
</svg>'

    # Check for SVG converter
    local converter=""
    if command -v rsvg-convert &>/dev/null; then
        converter="rsvg-convert"
    elif command -v magick &>/dev/null; then
        converter="magick"
    elif command -v convert &>/dev/null; then
        converter="convert"
    fi

    if [ -z "$converter" ]; then
        echo "⚠️  No SVG converter found. Install one of:"
        echo "   brew install librsvg    (recommended)"
        echo "   brew install imagemagick"
        return 1
    fi

    # Create temp SVG file
    SVG_FILE=$(mktemp /tmp/ssh_icon.XXXXXX.svg)
    echo "$SVG_CONTENT" > "$SVG_FILE"

    # Generate all required icon sizes
    local sizes=(16 32 64 128 256 512)
    for size in "${sizes[@]}"; do
        case $converter in
            rsvg-convert)
                rsvg-convert -w "$size" -h "$size" "$SVG_FILE" > "$ICONSET_DIR/icon_${size}x${size}.png"
                rsvg-convert -w "$((size*2))" -h "$((size*2))" "$SVG_FILE" > "$ICONSET_DIR/icon_${size}x${size}@2x.png"
                ;;
            magick|convert)
                $converter -background none -resize "${size}x${size}" "$SVG_FILE" "$ICONSET_DIR/icon_${size}x${size}.png"
                $converter -background none -resize "$((size*2))x$((size*2))" "$SVG_FILE" "$ICONSET_DIR/icon_${size}x${size}@2x.png"
                ;;
        esac
    done

    # Generate 1024x1024 for 512@2x
    case $converter in
        rsvg-convert)
            rsvg-convert -w 1024 -h 1024 "$SVG_FILE" > "$ICONSET_DIR/icon_512x512@2x.png"
            ;;
        magick|convert)
            $converter -background none -resize "1024x1024" "$SVG_FILE" "$ICONSET_DIR/icon_512x512@2x.png"
            ;;
    esac

    rm -f "$SVG_FILE"

    # Convert iconset to icns
    if ! iconutil -c icns "$ICONSET_DIR" -o "$RESOURCES_DIR/AppIcon.icns" 2>/dev/null; then
        echo "⚠️  iconutil failed, app will use default icon"
        rm -rf "$ICONSET_DIR"
        return 1
    fi

    rm -rf "$ICONSET_DIR"
    return 0
}

generate_icon || echo "   Continuing without custom icon..."

# Calculate app size
APP_SIZE=$(du -sh "$APP_DIR" | cut -f1)
BINARY_SIZE=$(du -sh "$BINARY_PATH" | cut -f1)

echo ""
echo "✅ Packaging complete!"
echo ""
echo "   App:     $APP_DIR"
echo "   Size:    $APP_SIZE (binary: $BINARY_SIZE)"
echo "   Version: $VERSION"
echo ""
echo "📋 Next steps:"
echo "   open \"$APP_DIR\"                    # Test the app"
echo "   cp -r \"$APP_DIR\" /Applications/    # Install"
