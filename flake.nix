{
  description = "Scratchmark - A pleasant Markdown editor";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        buildInputs = with pkgs; [
          # GTK4 & UI libraries
          gtk4
          libadwaita
          gtksourceview5

          # Glib & related
          glib
          gettext
          pcre2

          # Build tools
          meson
          ninja
          pkg-config
          glib-networking  # for gsettings
        ];

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
          glib
          meson
          ninja
          gettext
          desktop-file-utils
          appstream-glib
          gobject-introspection
          wrapGAppsHook4
        ];

        # Build the project using Meson
        scratchmarkPackage = pkgs.stdenv.mkDerivation {
          pname = "scratchmark";
          version = "1.8.0";

          src = ./.;

          inherit nativeBuildInputs buildInputs;

          # Rust needs HOME for cargo
          HOME = "$TMPDIR";

          # Set RUSTFLAGS for reproducible builds
          RUSTFLAGS = "--remap-path-prefix $NIX_BUILD_TOP=/build";

          mesonFlags = [
          ];

          # Skip failing checks for icon cache and desktop database
          postPatch = ''
            patchShebangs src/meson.build
            patchShebangs update_translations.sh
          '';

          postFixup = ''
            # Wrap the binary to find GSettings schemas
            wrapProgram $out/bin/scratchmark \
              --set GSETTINGS_SCHEMA_DIR "$out/share/gsettings-schemas/scratchmark-$version/glib-2.0/schemas" \
              --prefix XDG_DATA_DIRS : "$out/share"
          '';
        };

        # Create AppImage (uses system GTK4/libadwaita - no bundling of system libs)
        scratchmarkAppImage = pkgs.stdenv.mkDerivation {
          pname = "scratchmark-appimage";
          version = "1.8.0";

          src = ./.;

          nativeBuildInputs = with pkgs; [
            rustToolchain
            pkg-config
            glib
            meson
            ninja
            gettext
            desktop-file-utils
            appstream-glib
            squashfsTools
            patchelf
            file
            makeWrapper
          ];

          buildInputs = with pkgs; [
            gtk4
            libadwaita
            gtksourceview5
            glib
            gettext
            pcre2
          ];

          HOME = "$TMPDIR";
          RUSTFLAGS = "--remap-path-prefix $NIX_BUILD_TOP=/build";

          mesonFlags = [
          ];

          postPatch = ''
            patchShebangs src/meson.build
            patchShebangs update_translations.sh
          '';

          buildPhase = ''
            meson compile
          '';

          installPhase = ''
            mkdir -p $out/AppDir

            # Install the binary
            cp src/scratchmark $out/AppDir/usr/bin/scratchmark
            chmod +x $out/AppDir/usr/bin/scratchmark

            # Install resources
            cp src/scratchmark.gresource $out/AppDir/usr/share/scratchmark/

            # Install desktop file
            cp data/org.scratchmark.Scratchmark.desktop \
              $out/AppDir/usr/share/applications/org.scratchmark.Scratchmark.desktop

            # Install icons
            cp -r data/icons/* $out/AppDir/usr/share/icons/

            # Install GSettings schema
            mkdir -p $out/AppDir/usr/share/glib-2.0/schemas
            cp data/org.scratchmark.Scratchmark.gschema.xml \
              $out/AppDir/usr/share/glib-2.0/schemas/

            # Install metainfo
            cp data/org.scratchmark.Scratchmark.metainfo.xml \
              $out/AppDir/usr/share/metainfo/

            # Install AppRun script
            cp scripts/AppRun.in $out/AppDir/AppRun
            chmod +x $out/AppDir/AppRun

          # Create AppImage
          mksquashfs $out/AppDir $out/Scratchmark-$version-${system}.AppImage \
            -root-owned -noappend -comp xz -b 1M -Xdict-size 100%
          '';

          # Don't compress the AppImage again
          dontFixup = true;
        };

      in {
        packages = {
          default = scratchmarkPackage;
          scratchmark = scratchmarkPackage;
          appimage = scratchmarkAppImage;
        };

        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;

          # GSettings for development
          shellHook = ''
            export GSETTINGS_SCHEMA_DIR="${pkgs.glib.makeSchemaPath (placeholder "out") "scratchmark-gsettings"}"

            echo "🦀 Scratchmark Development Environment"
            echo ""
            echo "Available commands:"
            echo "  cargo run              # Run with Cargo (set GSETTINGS_SCHEMA_DIR=$PWD/data first)"
            echo "  meson setup build && cd build && meson compile"
            echo "  nix build .#scratchmark  # Build the package"
            echo "  nix build .#appimage      # Build AppImage (no system libs bundled)"
            echo ""
            echo "To push to Cachix:"
            echo "  cachix push <your-cache-name> ./result"
            echo ""
            echo "System requirements for AppImage users:"
            echo "  - gtk4"
            echo "  - libadwaita-1"
            echo "  - gtksourceview-5"
            echo ""
          '';
        };

        # Formatter
        formatter = pkgs.nixpkgs-fmt;
      }
    );
}
