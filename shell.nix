{ pkgs ? import <nixpkgs> {} }:

let
  isDarwin = pkgs.stdenv.isDarwin;
  isLinux = pkgs.stdenv.isLinux;
in

pkgs.mkShell {
  name = "wrldbldr-dev";

  buildInputs = with pkgs; [
    # Rust toolchain
    rustc
    cargo
    rustfmt
    clippy
    rust-analyzer

    # Build essentials
    pkg-config
    llvmPackages.lld  # Fast linker
    
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
    tmux  # Required by overmind

    # Frontend tooling (for Player web builds)
    dioxus-cli
    wasm-bindgen-cli
    binaryen       # wasm-opt

    # Node.js (for Tailwind CSS)
    nodejs_20
    nodePackages.npm
  ] 
  # macOS-specific dependencies
  # Note: Apple frameworks are automatically provided by the system SDK
  ++ lib.optionals isDarwin [
    libiconv
  ]
  # Linux-specific dependencies
  ++ lib.optionals isLinux [
    gcc
    binutils  # Provides ld
    
    # GTK and related libs (for Dioxus desktop)
    gtk3
    glib
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
  ];

  # Environment variables
  shellHook = ''
    # OpenSSL
    export OPENSSL_DIR="${pkgs.openssl.dev}"
    export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"
    export OPENSSL_INCLUDE_DIR="${pkgs.openssl.dev}/include"
    export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"

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
    
    # Ensure cargo binaries are in PATH
    export PATH="$HOME/.cargo/bin:$PATH"

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
  '';

  # Use lld for faster linking (optional, remove if causing issues)
  # CARGO_BUILD_RUSTFLAGS = "-C link-arg=-fuse-ld=lld";
}
