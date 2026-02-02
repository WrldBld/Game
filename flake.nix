{
  description = "WrldBldr development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix.url = "github:nix-community/fenix";
  };

  outputs = { self, nixpkgs, flake-utils, fenix }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
        lib = pkgs.lib;
        isDarwin = pkgs.stdenv.isDarwin;
        isLinux = pkgs.stdenv.isLinux;
        rustToolchain = fenix.packages.${system}.stable.withComponents [
          "cargo"
          "rustc"
          "rustfmt"
          "clippy"
          "rust-analyzer"
          "rust-src"
        ];

        # KoboldCpp package for macOS Apple Silicon with Metal support
        # Pinned to v1.107.1 - latest stable release as of 2026-02-02
        # This revision includes Vulkan improvements, Flux2 support, and SDXS image generation
        koboldcpp = pkgs.stdenv.mkDerivation rec {
          pname = "koboldcpp";
          version = "1.107.1";

          src = pkgs.fetchFromGitHub {
            owner = "LostRuins";
            repo = "koboldcpp";
            rev = "v${version}";
            sha256 = "sha256-dmFwWPD+S4nQ8p3DFnzplQ0lN8EGCi4VhENiDYIadxw=";
          };

          nativeBuildInputs = with pkgs; [
            makeWrapper
          ] ++ lib.optionals isDarwin [
            # Metal framework is provided by macOS system SDK
          ];

          buildInputs = with pkgs; [
            python3
          ] ++ lib.optionals isDarwin [
            # No additional build inputs needed for Metal - uses system frameworks
          ];

          # Build flags for Apple Silicon with Metal GPU acceleration
          # LLAMA_METAL=1 enables Metal GPU support (Apple Silicon)
          # LLAMA_PORTABLE=1 makes the build sharable across different devices
          makeFlags = lib.optionals isDarwin [
            "LLAMA_METAL=1"
            "LLAMA_PORTABLE=1"
          ];

          enableParallelBuilding = true;

          installPhase = ''
            runHook preInstall

            mkdir -p $out/bin
            mkdir -p $out/lib
            mkdir -p $out/share/koboldcpp

            # Copy the main Python script
            cp koboldcpp.py $out/bin/koboldcpp
            chmod +x $out/bin/koboldcpp

            # Copy built libraries
            cp *.so $out/lib/ 2>/dev/null || true
            cp *.dylib $out/lib/ 2>/dev/null || true

            # Copy supporting files
            cp -r klite.embd $out/share/koboldcpp/ 2>/dev/null || true
            cp -r rwkv_vocab $out/share/koboldcpp/ 2>/dev/null || true
            cp -r rwkv_stvenv $out/share/koboldcpp/ 2>/dev/null || true

            # Copy Metal shader source files required for runtime compilation
            mkdir -p $out/share/koboldcpp/ggml-metal
            cp ggml/src/ggml-metal/ggml-metal.metal $out/share/koboldcpp/ggml-metal/ 2>/dev/null || true
            cp ggml/src/ggml-metal/ggml-metal-impl.h $out/share/koboldcpp/ggml-metal/ 2>/dev/null || true
            cp ggml/include/ggml-metal.h $out/share/koboldcpp/ggml-metal/ 2>/dev/null || true

            # Copy ggml-common.h which is included by ggml-metal-merged.metal
            cp ggml/src/ggml-common.h $out/share/koboldcpp/ggml-metal/ 2>/dev/null || true

            # Some versions of KoboldCpp expect ggml-metal-merged.metal with inlined headers
            # Generate it if upstream doesn't provide one
            if [[ -f "ggml/src/ggml-metal/ggml-metal-merged.metal" ]]; then
                cp ggml/src/ggml-metal/ggml-metal-merged.metal $out/share/koboldcpp/ggml-metal/ 2>/dev/null || true
            else
                OUT_DIR="$out" python3 - <<'PY'
import os
import pathlib

root = pathlib.Path(".")
metal_path = root / "ggml/src/ggml-metal/ggml-metal.metal"
common_path = root / "ggml/src/ggml-common.h"
impl_path = root / "ggml/src/ggml-metal/ggml-metal-impl.h"

metal = metal_path.read_text()
common = common_path.read_text()
impl = impl_path.read_text()

block = '#if defined(GGML_METAL_EMBED_LIBRARY)\n__embed_ggml-common.h__\n#else\n#include "ggml-common.h"\n#endif'
if block in metal:
    metal = metal.replace(block, common)

metal = metal.replace('#include "ggml-metal-impl.h"', impl)

out_dir = pathlib.Path(os.environ["OUT_DIR"]) / "share/koboldcpp/ggml-metal"
out_dir.mkdir(parents=True, exist_ok=True)
out_path = out_dir / "ggml-metal-merged.metal"
out_path.write_text(metal)
PY
            fi

            # Wrap the binary to find libraries and Metal shaders
            wrapProgram $out/bin/koboldcpp \
              --prefix PYTHONPATH : "$out/share/koboldcpp" \
              --prefix LD_LIBRARY_PATH : "$out/lib" \
              --prefix DYLD_LIBRARY_PATH : "$out/lib" \
              --set GGML_METAL_PATH_RESOURCES "$out/share/koboldcpp/ggml-metal"

            runHook postInstall
          '';

          meta = with lib; {
            description = "Easy-to-use AI text-generation software for GGML and GGUF models";
            homepage = "https://github.com/LostRuins/koboldcpp";
            license = licenses.agpl3Only;
            platforms = platforms.darwin;
            mainProgram = "koboldcpp";
          };
        };
      in {
        packages = lib.optionalAttrs isDarwin {
          inherit koboldcpp;
        };

        devShells.default = pkgs.mkShell {
          name = "wrldbldr-dev";

          packages = [
            rustToolchain
          ] ++ (with pkgs; [
            # Build essentials
            pkg-config
            llvmPackages.lld

            # OpenSSL (for reqwest, neo4rs, etc.)
            openssl
            openssl.dev

            # SQLite (for sqlx)
            sqlite

            # Task runner
            go-task

            # Code statistics
            tokei

            # Process manager for running multiple services
            overmind
            tmux

            # Frontend tooling (for Player web builds)
            dioxus-cli
            wasm-bindgen-cli
            binaryen

            # Node.js (for Tailwind CSS)
            nodejs_20
            nodePackages.npm

            # Python (for repo tooling / scripts)
            python3
            python3Packages.pip
            python3Packages.virtualenv
          ])
          # macOS-specific dependencies
          # Note: Apple frameworks are automatically provided by the system SDK
          ++ lib.optionals isDarwin [
            pkgs.libiconv
            koboldcpp
            pkgs.screen
          ]
          # Linux-specific dependencies
          ++ lib.optionals isLinux (with pkgs; [
            gcc
            binutils

            # GTK and related libs (for Dioxus desktop)
            gtk3
            glib
            glib-networking
            cairo
            pango
            gdk-pixbuf
            atk
            webkitgtk_4_1
            libsoup_3

            # Wayland support
            wayland
            wayland-protocols
            libxkbcommon

            # X11 support (fallback)
            xorg.libX11
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi

            # Additional libs
            dbus
            at-spi2-atk
          ]);

          # Environment variables
          shellHook = ''
            # OpenSSL
            export OPENSSL_DIR="${pkgs.openssl.dev}"
            export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"
            export OPENSSL_INCLUDE_DIR="${pkgs.openssl.dev}/include"
            export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"

            # Rust analyzer stdlib sources
            export RUST_SRC_PATH="${rustToolchain}/lib/rustlib/src/rust/library"

            # SQLite
            export SQLITE3_LIB_DIR="${pkgs.sqlite.out}/lib"

            ${if isLinux then ''
            # GTK/GLib for Dioxus desktop (Linux only)
            export GIO_MODULE_DIR="${pkgs.glib-networking}/lib/gio/modules"
            export GIO_EXTRA_MODULES="${pkgs.glib-networking}/lib/gio/modules"

            # WebKit
            export WEBKIT_DISABLE_COMPOSITING_MODE=1

            # Wayland/X11
            export LD_LIBRARY_PATH="${pkgs.wayland}/lib:${pkgs.libxkbcommon}/lib:$LD_LIBRARY_PATH"
            '' else ""}

            ${if isDarwin then ''
            # macOS-specific library paths
            export DYLD_LIBRARY_PATH="${pkgs.openssl.out}/lib:${pkgs.sqlite.out}/lib:''${DYLD_LIBRARY_PATH:-}"

            # macOS frameworks are automatically linked via nix-darwin
            '' else ""}

            # Ensure user cargo bins don't shadow nix toolchain
            export PATH="$PATH:$HOME/.cargo/bin"

            echo "WrldBldr development environment loaded!"
            echo "Platform: ${if isDarwin then "macOS (nix-darwin)" else "Linux"}"
            echo ""
            echo "Available tasks:"
            echo "  task backend     - Run the Engine backend"
            echo "  task web:dev     - Run the Player frontend (web/WASM)"
            echo "  task dev         - Run both backend and frontend"
            echo "  task check       - Check all crates"
            echo "  task build       - Build all crates"
            echo ""
            echo "LLM Serving (macOS Apple Silicon):"
            echo "  ./scripts/serve-llm.sh              - Serve local LLM with OpenAI-compatible API"
            echo "  ./scripts/serve-llm.sh --help       - Show LLM server options"
            echo ""
            echo "KoboldCpp (via Nix):"
            echo "  koboldcpp --help                    - Show KoboldCpp options"
            echo "  koboldcpp --model model.gguf        - Run KoboldCpp with a model"
            echo "  ./scripts/koboldcpp-screen.sh       - Start KoboldCpp in screen session"
            echo "  task koboldcpp:screen               - Start KoboldCpp in background"
            echo ""
          '';
        };
      }
    );
}
