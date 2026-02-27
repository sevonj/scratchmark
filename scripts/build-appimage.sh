#!/usr/bin/bash
# Build Scratchmark AppImage and optionally push to Cachix

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Parse arguments
CACHE_NAME=""
while [[ $# -gt 0 ]]; do
  case $1 in
    --cache|-c)
      CACHE_NAME="$2"
      shift 2
      ;;
    --help|-h)
      echo "Usage: $0 [--cache CACHE_NAME]"
      echo ""
      echo "Build Scratchmark AppImage using Nix."
      echo ""
      echo "Options:"
      echo "  --cache, -c CACHE_NAME    Push build to Cachix after building"
      echo "  --help, -h                Show this help message"
      exit 0
      ;;
    *)
      echo -e "${RED}Unknown option: $1${NC}"
      echo "Use --help for usage information"
      exit 1
      ;;
  esac
done

# Get version from Cargo.toml
VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)

echo -e "${GREEN}Building Scratchmark AppImage v${VERSION}...${NC}"
echo ""

# Check if Nix with flakes is available
if ! command -v nix &> /dev/null; then
    echo -e "${RED}Error: Nix not found. Please install Nix first.${NC}"
    exit 1
fi

# Build the AppImage
echo "Building with Nix..."
nix build .#appimage --print-build-logs

# Find and copy the AppImage to a more convenient location
APPIMAGE=$(find result -name "*.AppImage" -type f | head -1)
if [ -z "$APPIMAGE" ]; then
    echo -e "${RED}Error: AppImage not found in build output${NC}"
    exit 1
fi

OUTPUT_DIR="dist"
mkdir -p "$OUTPUT_DIR"
OUTPUT="$OUTPUT_DIR/Scratchmark-${VERSION}-x86_64.AppImage"

cp "$APPIMAGE" "$OUTPUT"
chmod +x "$OUTPUT"

echo ""
echo -e "${GREEN}✓ AppImage built successfully!${NC}"
echo "Location: $OUTPUT"
echo "Size: $(du -h "$OUTPUT" | cut -f1)"

# Calculate checksum
CHECKSUM=$(sha256sum "$OUTPUT" | cut -d' ' -f1)
echo "SHA256: $CHECKSUM"

# Save checksum to file
echo "$CHECKSUM  $(basename "$OUTPUT")" > "$OUTPUT_DIR/sha256sums.txt"

# Push to Cachix if requested
if [ -n "$CACHE_NAME" ]; then
    echo ""
    echo -e "${YELLOW}Pushing to Cachix cache: $CACHE_NAME${NC}"

    if ! command -v cachix &> /dev/null; then
        echo -e "${RED}Error: cachix not found. Install it with:"
        echo "  nix-env -iA nixpkgs.cachix"
        echo "  cachix use $CACHE_NAME"
        exit 1
    fi

    cachix push "$CACHE_NAME" ./result
    echo -e "${GREEN}✓ Pushed to Cachix${NC}"
fi

echo ""
echo -e "${GREEN}To test the AppImage:${NC}"
echo "  $OUTPUT"
echo ""
echo -e "${GREEN}To distribute:${NC}"
echo "  - Upload $OUTPUT to GitHub Releases"
echo "  - Share the SHA256 checksum for verification"
echo ""
