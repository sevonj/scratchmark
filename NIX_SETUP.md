# Nix + AppImage Setup for Scratchmark

This document explains how to build Scratchmark with Nix and distribute it as an AppImage without requiring Nix on end-user machines.

## Quick Start

### Build the AppImage locally

```bash
# Using Nix directly
nix build .#appimage

# Or use the helper script
./scripts/build-appimage.sh
```

The AppImage will be created in `./dist/` with the current version.

### Build and push to Cachix

```bash
./scripts/build-appimage.sh --cache your-cache-name
```

## File Structure

```
.
├── flake.nix                    # Nix flake with build definitions
├── .github/workflows/
│   └── appimage.yml            # GitHub Actions to build AppImage on tag push
├── scripts/
│   ├── build-appimage.sh       # Helper script to build AppImage
│   └── README.md               # Documentation for scripts
├── dist/                       # Created by build script (gitignored)
│   ├── Scratchmark-1.8.0-x86_64.AppImage
│   └── sha256sums.txt
└── README.md                   # Updated with AppImage section
```

## Distribution Workflow

### Option 1: Automated GitHub Releases

1. Tag a new version:
   ```bash
   git tag v1.8.0
   git push origin v1.8.0
   ```

2. GitHub Actions automatically:
   - Builds the AppImage
   - Creates a GitHub Release
   - Uploads the AppImage as an artifact

3. Users download and run:
   ```bash
   chmod +x Scratchmark-1.8.0-x86_64.AppImage
   ./Scratchmark-1.8.0-x86_64.AppImage
   ```

### Option 2: Manual Build

1. Build locally:
   ```bash
   ./scripts/build-appimage.sh
   ```

2. Upload `dist/Scratchmark-{version}-x86_64.AppImage` to:
   - GitHub Releases
   - Your website
   - Any file hosting service

3. Provide installation instructions (see below)

## User Installation Instructions

### Download

Users download the AppImage from your GitHub Releases or website.

### System Requirements

Users need these libraries installed (no Nix required):

**Ubuntu/Debian:**
```bash
sudo apt install libgtk-4-1 libadwaita-1-0 libgtksourceview-5-0
```

**Fedora:**
```bash
sudo dnf install gtk4 libadwaita gtksourceview5
```

**Arch Linux:**
```bash
sudo pacman -S gtk4 libadwaita gtksourceview5
```

**openSUSE:**
```bash
sudo zypper install gtk4 libadwaita gtksourceview5
```

### Run

```bash
# Make executable
chmod +x Scratchmark-1.8.0-x86_64.AppImage

# Run
./Scratchmark-1.8.0-x86_64.AppImage
```

## Cachix Integration (Optional but Recommended)

### Why Use Cachix?

- Share pre-built binaries across machines
- Faster rebuilds for CI/CD
- Free tier available

### Setup

1. Create account at https://cachix.org
2. Create a new cache
3. Install cachix CLI:
   ```bash
   nix-env -iA nixpkgs.cachix
   ```
4. Authenticate:
   ```bash
   cachix use your-cache-name
   cachix authtoken YOUR_TOKEN  # Get from Cachix dashboard
   ```

### Use in CI

Add to your GitHub Actions workflow:

```yaml
- name: Setup Cachix
  uses: cachix/cachix-action@v12
  with:
    name: your-cache-name
    authToken: '${{ secrets.CACHIX_AUTH_TOKEN }}'

- name: Build AppImage
  run: nix build .#appimage --print-build-logs

- name: Push to Cachix
  run: cachix push your-cache-name ./result
```

### Use on Other Machines

```bash
cachix use your-cache-name
nix build .#appimage  # Pulls pre-built artifacts instantly
```

## Nix Flake Commands

```bash
# Show available packages
nix flake show

# Build the default package
nix build

# Build just the binary (no AppImage)
nix build .#scratchmark

# Build AppImage
nix build .#appimage

# Enter development shell
nix develop

# Format the flake
nix fmt

# Update flake inputs
nix flake update
```

## Troubleshooting

### "Command not found: nix"

Install Nix: https://nixos.org/download.html

```bash
sh <(curl -L https://nixos.org/nix/install) --daemon
```

### "experimental-features not enabled"

Add to `~/.config/nix/nix.conf`:
```
experimental-features = nix-command flakes
```

### AppImage fails to launch

Check that system dependencies are installed:
```bash
# Check GTK4
ldd ./dist/Scratchmark-*.AppImage | grep gtk
```

## Technical Details

### AppImage Structure

The AppImage contains:
- Scratchmark binary (Rust executable)
- GResources bundle (UI templates, icons, etc.)
- Desktop file
- AppRun wrapper script
- GSettings schema
- Icons

The AppImage **does not bundle** system libraries (GTK4, libadwaita, etc.) to keep it smaller.

### AppRun Script

The `AppRun` script sets up the environment:
```bash
export XDG_DATA_DIRS="$HERE/usr/share:$XDG_DATA_DIRS"
export GSETTINGS_SCHEMA_DIR="$HERE/usr/share/glib-2.0/schemas"
exec "$HERE/usr/bin/scratchmark" "$@"
```

### Why Not Bundle System Libraries?

- **Smaller size**: ~50MB vs ~150MB+ with bundled libs
- **Security**: System gets security updates automatically
- **Updates**: Users benefit from system GTK4 improvements
- **Simplicity**: Easier to maintain builds

## Next Steps

1. Test building locally:
   ```bash
   ./scripts/build-appimage.sh
   ```

2. Test the AppImage:
   ```bash
   ./dist/Scratchmark-1.8.0-x86_64.AppImage
   ```

3. Set up Cachix (optional but recommended):
   - Create account
   - Install CLI
   - Test pushing build

4. Create a test tag to verify GitHub Actions:
   ```bash
   git tag v0.0.1-test
   git push origin v0.0.1-test
   ```

5. Check the GitHub Actions run and the created release

6. Delete test tag when satisfied:
   ```bash
   git tag -d v0.0.1-test
   git push origin :refs/tags/v0.0.1-test
   ```

## Support

For issues with:
- **Nix**: See https://nixos.org/manual/nix/stable/
- **AppImage**: See https://docs.appimage.org/
- **Cachix**: See https://cachix.org/docs/
