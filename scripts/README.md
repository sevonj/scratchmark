# Build Scripts

This directory contains helper scripts for building and distributing Scratchmark.

## build-appimage.sh

Build a Scratchmark AppImage using Nix.

### Usage

```bash
# Just build the AppImage
./scripts/build-appimage.sh

# Build and push to Cachix (requires cachix setup)
./scripts/build-appimage.sh --cache your-cache-name

# Show help
./scripts/build-appimage.sh --help
```

### What it does

1. Builds the AppImage using Nix flake
2. Copies the AppImage to `dist/` directory
3. Generates a SHA256 checksum in `dist/sha256sums.txt`
4. Optionally pushes the build to your Cachix cache

### Output

After running, you'll find:
- `dist/Scratchmark-{version}-x86_64.AppImage` - The AppImage
- `dist/sha256sums.txt` - Checksum for verification

### System Requirements for Users

Users who download the AppImage need these libraries:
- `gtk4`
- `libadwaita-1`
- `gtksourceview-5`

On Ubuntu/Debian:
```bash
sudo apt install libgtk-4-1 libadwaita-1-0 libgtksourceview-5-0
```

On Fedora:
```bash
sudo dnf install gtk4 libadwaita gtksourceview5
```

On Arch Linux:
```bash
sudo pacman -S gtk4 libadwaita gtksourceview5
```

### Cachix Integration

To set up Cachix:

1. Create a free account at https://cachix.org
2. Create a new cache
3. Install the CLI:
   ```bash
   nix-env -iA nixpkgs.cachix
   cachix use your-cache-name
   ```
4. Set up authentication (run `cachix authtoken` after signing in)
5. Now you can build and push:
   ```bash
   ./scripts/build-appimage.sh --cache your-cache-name
   ```

On other machines, use the cache:
```bash
cachix use your-cache-name
nix build .#appimage  # Will pull pre-built artifacts
```
