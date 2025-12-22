{
  description = "WrldBldr Game Development Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        # Detect platform
        isLinux = pkgs.stdenv.isLinux;
        isDarwin = pkgs.stdenv.isDarwin;
        
        # Rust toolchain - stable with wasm32 target and tools
        # Matches rust-toolchain.toml: channel = "stable", targets = ["wasm32-unknown-unknown"]
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "wasm32-unknown-unknown" ];
          extensions = [ "rustfmt" "clippy" ];
        };
        
        # Base packages needed on all platforms
        basePackages = with pkgs; [
          # Rust toolchain
          rustToolchain
          rust-analyzer
          
          # Build essentials
          gcc
          binutils
          pkg-config
          
          # OpenSSL (for reqwest, neo4rs, etc.)
          openssl
          openssl.dev
          
          # SQLite (for sqlx)
          sqlite
          
          # Task runner
          go-task
          
          # Process manager for running multiple services
          overmind
          tmux  # Required by overmind
          
          # Frontend tooling (for Player web builds)
          dioxus-cli
          wasm-bindgen-cli
          binaryen  # wasm-opt
          
          # Node.js (for Tailwind CSS)
          nodejs_20
          nodePackages.npm
        ];
        
        # Linux-specific packages (GTK for Dioxus desktop)
        linuxPackages = with pkgs; lib.optionals isLinux [
          gtk3
          glib
          cairo
          pango
          gdk-pixbuf
          atk
          webkitgtk_4_1
          libsoup_3
          wayland
          wayland-protocols
          libxkbcommon
          xorg.libX11
          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi
          dbus
          at-spi2-atk
        ];
        
        # macOS-specific packages
        darwinPackages = with pkgs; lib.optionals isDarwin [
          # macOS frameworks are typically available via system, but we might need:
          libiconv
          darwin.apple_sdk.frameworks.Security
          darwin.apple_sdk.frameworks.CoreFoundation
        ];
        
        allPackages = basePackages ++ linuxPackages ++ darwinPackages;
        
        # Environment variables for shell hook
        shellHook = ''
          # OpenSSL
          export OPENSSL_DIR="${pkgs.openssl.dev}"
          export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"
          export OPENSSL_INCLUDE_DIR="${pkgs.openssl.dev}/include"
          export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"
          
          # SQLite
          export SQLITE3_LIB_DIR="${pkgs.sqlite.out}/lib"
        '' + pkgs.lib.optionalString isLinux ''
          # GTK/GLib for Dioxus desktop (Linux only)
          export GIO_MODULE_DIR="${pkgs.glib-networking}/lib/gio/modules"
          export GIO_EXTRA_MODULES="${pkgs.glib-networking}/lib/gio/modules"
          export WEBKIT_DISABLE_COMPOSITING_MODE=1
          export LD_LIBRARY_PATH="${pkgs.wayland}/lib:${pkgs.libxkbcommon}/lib:$LD_LIBRARY_PATH"
        '' + pkgs.lib.optionalString isDarwin ''
          # macOS-specific environment variables
          export DYLD_FRAMEWORK_PATH="${pkgs.darwin.apple_sdk.frameworks.Security}/Library/Frameworks:${pkgs.darwin.apple_sdk.frameworks.CoreFoundation}/Library/Frameworks"
        '' + ''
          # Ensure cargo binaries are in PATH
          export PATH="$HOME/.cargo/bin:$PATH"
          
          echo "WrldBldr development environment loaded!"
          echo "Platform: ${system}"
          echo ""
          echo "Available tasks:"
          echo "  task backend     - Run the Engine backend"
          echo "  task web:dev     - Run the Player frontend (web/WASM)"
          echo "  task dev         - Run both backend and frontend"
          echo "  task check       - Check all crates"
          echo "  task build       - Build all crates"
          echo ""
        '';
      in
      {
        devShells.default = pkgs.mkShell {
          name = "wrldbldr-dev";
          buildInputs = allPackages;
          shellHook = shellHook;
        };
      }
    );
}


