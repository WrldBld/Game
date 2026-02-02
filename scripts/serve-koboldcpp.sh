#!/usr/bin/env bash
# KoboldCpp Server for GGUF models on macOS Apple Silicon
# Serves local LLMs with an OpenAI-compatible API using Metal GPU acceleration
#
# This script prefers a KoboldCpp executable from Nix (PATH) and falls back to
# cloning and building from source if not available.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Default configuration
DEFAULT_MODEL="${KOBOLDCPP_MODEL:-$PROJECT_ROOT/models/Noromaid-20B-v0.3-Q4_K_M.gguf}"
PORT="${KOBOLDCPP_PORT:-5001}"
HOST="${KOBOLDCPP_HOST:-127.0.0.1}"
KOBOLDCPP_DIR="${KOBOLDCPP_DIR:-$PROJECT_ROOT/tools/koboldcpp}"
KOBOLDCPP_BIN="${KOBOLDCPP_BIN:-}"
EXTRA_ARGS="${KOBOLDCPP_ARGS:-}"

# Check if KoboldCpp is available via Nix (PATH)
KOBOLDCPP_FROM_PATH=""
if command -v koboldcpp >/dev/null 2>&1; then
    KOBOLDCPP_FROM_PATH="$(command -v koboldcpp)"
fi

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_debug() { echo -e "${BLUE}[DEBUG]${NC} $1"; }

# Check we're on macOS with Apple Silicon
check_platform() {
    if [[ "$(uname)" != "Darwin" ]]; then
        log_error "This script is optimized for macOS. KoboldCpp supports Linux and Windows too,"
        log_error "but this script focuses on Apple Silicon Metal acceleration."
        exit 1
    fi

    if [[ "$(uname -m)" != "arm64" ]]; then
        log_warn "This script is optimized for Apple Silicon (arm64)."
        log_warn "Intel Macs are supported but Metal acceleration may not be available."
    else
        log_info "Platform check passed: macOS on Apple Silicon"
    fi
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check for required dependencies
check_dependencies() {
    local missing_deps=()

    if ! command_exists git; then
        missing_deps+=("git")
    fi

    if ! command_exists make; then
        missing_deps+=("make")
    fi

    if ! command_exists python3; then
        missing_deps+=("python3")
    fi

    if [[ ${#missing_deps[@]} -gt 0 ]]; then
        log_error "Missing required dependencies: ${missing_deps[*]}"
        log_error "Please install Xcode Command Line Tools:"
        log_error "  xcode-select --install"
        exit 1
    fi

    log_info "All dependencies satisfied"
}

# Clone KoboldCpp repository if not present
clone_koboldcpp() {
    if [[ ! -d "$KOBOLDCPP_DIR" ]]; then
        log_info "KoboldCpp not found. Cloning into $KOBOLDCPP_DIR..."
        mkdir -p "$(dirname "$KOBOLDCPP_DIR")"
        git clone --depth=1 https://github.com/LostRuins/koboldcpp.git "$KOBOLDCPP_DIR"
        log_info "KoboldCpp cloned successfully"
    else
        log_info "KoboldCpp found at $KOBOLDCPP_DIR"
    fi
}

# Build KoboldCpp with Metal support for Apple Silicon
build_koboldcpp() {
    log_info "Building KoboldCpp with Metal acceleration..."
    cd "$KOBOLDCPP_DIR"

    # For Apple Silicon, use Metal GPU support
    # Per upstream docs: make LLAMA_METAL=1
    # LLAMA_PORTABLE=1 makes the build sharable with other devices
    if [[ "$(uname -m)" == "arm64" ]]; then
        log_info "Building with Metal GPU acceleration (LLAMA_METAL=1)..."
        make LLAMA_METAL=1 -j$(sysctl -n hw.ncpu)
    else
        log_info "Building for Intel Mac (CPU-only)..."
        make -j$(sysctl -n hw.ncpu)
    fi

    log_info "Build completed successfully"
}

# Find the KoboldCpp binary (prefer Nix, then explicit, then built)
find_binary() {
    # Prefer KoboldCpp from Nix (PATH) if available
    if [[ -n "$KOBOLDCPP_FROM_PATH" ]]; then
        KOBOLDCPP_BIN="$KOBOLDCPP_FROM_PATH"
        log_info "Using KoboldCpp from Nix: $KOBOLDCPP_BIN"
        return 0
    fi

    # If KOBOLDCPP_BIN is explicitly set and exists, use it
    if [[ -n "$KOBOLDCPP_BIN" && -f "$KOBOLDCPP_BIN" ]]; then
        log_info "Using prebuilt binary: $KOBOLDCPP_BIN"
        return 0
    fi

    # Look for the built library (macOS uses .so extension for koboldcpp)
    local lib_paths=(
        "$KOBOLDCPP_DIR/koboldcpp_default.so"
        "$KOBOLDCPP_DIR/koboldcpp.so"
    )

    for path in "${lib_paths[@]}"; do
        if [[ -f "$path" ]]; then
            KOBOLDCPP_BIN="$path"
            log_info "Found built library: $KOBOLDCPP_BIN"
            return 0
        fi
    done

    return 1
}

# Setup KoboldCpp (clone and/or build)
setup_koboldcpp() {
    clone_koboldcpp

    if ! find_binary; then
        log_info "Binary not found, building from source..."
        build_koboldcpp
        if ! find_binary; then
            log_error "Build succeeded but binary not found. Check $KOBOLDCPP_DIR for output files."
            exit 1
        fi
    fi
}

# Check if model file exists
check_model() {
    local model_path="$1"

    if [[ ! -f "$model_path" ]]; then
        log_error "Model file not found: $model_path"
        log_info "Please download a GGUF model and place it at the specified path,"
        log_info "or set KOBOLDCPP_MODEL to point to an existing model file."
        log_info ""
        log_info "Example models:"
        log_info "  - Noromaid 20B: https://huggingface.co/TheBloke/Noromaid-20B-v0.3-GGUF"
        log_info "  - Llama 3.1 8B: https://huggingface.co/bartowski/Meta-Llama-3.1-8B-Instruct-GGUF"
        log_info "  - Mistral 7B:   https://huggingface.co/TheBloke/Mistral-7B-Instruct-v0.3-GGUF"
        exit 1
    fi

    log_info "Model file found: $model_path"
}

# Print usage
usage() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS] [MODEL_PATH]

Serve GGUF models locally using KoboldCpp with Metal GPU acceleration on macOS.

Arguments:
    MODEL_PATH           Path to GGUF model file (default: $DEFAULT_MODEL)

Options:
    -p, --port PORT      Port to listen on (default: $PORT, env: KOBOLDCPP_PORT)
    -H, --host HOST      Host to bind to (default: $HOST, env: KOBOLDCPP_HOST)
    -g, --gpulayers N    Number of layers to offload to GPU (default: auto)
    -c, --contextsize N  Context size in tokens (default: 4096)
    --build              Force rebuild of KoboldCpp
    -h, --help           Show this help message

Environment Variables:
    KOBOLDCPP_MODEL      Default model path
    KOBOLDCPP_HOST       Host to bind to
    KOBOLDCPP_PORT       Port to listen on
    KOBOLDCPP_DIR        Directory for KoboldCpp source (default: $KOBOLDCPP_DIR)
    KOBOLDCPP_BIN        Path to prebuilt binary (skips build)
    KOBOLDCPP_ARGS       Additional arguments passed to KoboldCpp

Nix Integration:
    If KoboldCpp is installed via Nix (available in PATH), this script will
    use it automatically, skipping the clone/build step. To use Nix:
        nix develop  # or direnv allow
    The script prefers Nix-installed KoboldCpp over source-built versions.

Examples:
    # Serve default model
    $(basename "$0")

    # Serve specific model
    $(basename "$0") /path/to/model.gguf

    # Custom port and GPU layers
    $(basename "$0") -p 8080 -g 35 model.gguf

    # Large context window
    $(basename "$0") -c 8192 model.gguf

    # Using environment variables
    KOBOLDCPP_MODEL=/models/llama.gguf KOBOLDCPP_PORT=8080 $(basename "$0")

Build Commands (from upstream docs):
    # Apple Silicon with Metal:
    make LLAMA_METAL=1

    # Portable build (sharable with other devices):
    make LLAMA_METAL=1 LLAMA_PORTABLE=1

    # Intel Mac (CPU only):
    make

The server provides an OpenAI-compatible API at:
    http://HOST:PORT/api/v1/generate
    http://HOST:PORT/v1/chat/completions
    http://HOST:PORT/v1/completions

KoboldCpp Repository:
    https://github.com/LostRuins/koboldcpp
EOF
}

# Parse command line arguments
parse_args() {
    GPU_LAYERS=""
    CONTEXT_SIZE=""
    FORCE_BUILD=false

    while [[ $# -gt 0 ]]; do
        case $1 in
            -p|--port)
                PORT="$2"
                shift 2
                ;;
            -H|--host)
                HOST="$2"
                shift 2
                ;;
            -g|--gpulayers)
                GPU_LAYERS="$2"
                shift 2
                ;;
            -c|--contextsize)
                CONTEXT_SIZE="$2"
                shift 2
                ;;
            --build)
                FORCE_BUILD=true
                shift
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            -*)
                log_error "Unknown option: $1"
                usage
                exit 1
                ;;
            *)
                # Assume it's the model path
                DEFAULT_MODEL="$1"
                shift
                ;;
        esac
    done
}

# Start the KoboldCpp server
start_server() {
    local model_path="$1"

    log_info "Starting KoboldCpp server..."
    log_info "Model: $model_path"
    log_info "Host: $HOST"
    log_info "Port: $PORT"

    local args="--model $model_path --host $HOST --port $PORT"

    if [[ -n "$GPU_LAYERS" ]]; then
        args="$args --gpulayers $GPU_LAYERS"
        log_info "GPU Layers: $GPU_LAYERS"
    else
        log_info "GPU Layers: auto (Metal will use all available)"
    fi

    if [[ -n "$CONTEXT_SIZE" ]]; then
        args="$args --contextsize $CONTEXT_SIZE"
        log_info "Context Size: $CONTEXT_SIZE"
    fi

    if [[ -n "$EXTRA_ARGS" ]]; then
        args="$args $EXTRA_ARGS"
        log_info "Extra Args: $EXTRA_ARGS"
    fi

    echo ""
    log_info "Starting server with command:"

    # Check if using Nix-installed KoboldCpp
    if [[ "$KOBOLDCPP_BIN" == "$KOBOLDCPP_FROM_PATH" ]]; then
        # Using Nix-installed KoboldCpp - GGML_METAL_PATH_RESOURCES is set by wrapper
        log_debug "koboldcpp $args"
        echo ""
        exec koboldcpp $args
    else
        # Using source-built KoboldCpp
        log_debug "python3 $KOBOLDCPP_DIR/koboldcpp.py $args"
        echo ""

        # Change to koboldcpp directory so it can find its libraries
        cd "$KOBOLDCPP_DIR"

        # Set Metal shader path for source builds to avoid ggml-metal.metal not found error
        export GGML_METAL_PATH_RESOURCES="$KOBOLDCPP_DIR/ggml/src/ggml-metal"

        # Some versions of KoboldCpp expect ggml-metal-merged.metal with inlined headers
        # Generate it if it doesn't exist
        if [[ ! -f "$GGML_METAL_PATH_RESOURCES/ggml-metal-merged.metal" ]]; then
            local metal_src="$KOBOLDCPP_DIR/ggml/src/ggml-metal/ggml-metal.metal"
            local common_src="$KOBOLDCPP_DIR/ggml/src/ggml-common.h"
            local impl_src="$KOBOLDCPP_DIR/ggml/src/ggml-metal/ggml-metal-impl.h"

            if [[ -f "$metal_src" && -f "$common_src" && -f "$impl_src" ]]; then
                log_warn "ggml-metal-merged.metal not found, generating merged file"
                METAL_OUT="$GGML_METAL_PATH_RESOURCES/ggml-metal-merged.metal" \
                METAL_SRC="$metal_src" \
                COMMON_SRC="$common_src" \
                IMPL_SRC="$impl_src" \
                python3 - <<'PY'
import os
import pathlib

metal = pathlib.Path(os.environ["METAL_SRC"]).read_text()
common = pathlib.Path(os.environ["COMMON_SRC"]).read_text()
impl = pathlib.Path(os.environ["IMPL_SRC"]).read_text()

block = '#if defined(GGML_METAL_EMBED_LIBRARY)\n__embed_ggml-common.h__\n#else\n#include "ggml-common.h"\n#endif'
if block in metal:
    metal = metal.replace(block, common)

metal = metal.replace('#include "ggml-metal-impl.h"', impl)

out_path = pathlib.Path(os.environ["METAL_OUT"])
out_path.parent.mkdir(parents=True, exist_ok=True)
out_path.write_text(metal)
PY
            fi
        fi

        # ggml-common.h is included by ggml-metal-merged.metal but may not be in the same directory
        # Copy it from ggml/src if missing in the Metal resources directory
        if [[ ! -f "$GGML_METAL_PATH_RESOURCES/ggml-common.h" ]]; then
            if [[ -f "$KOBOLDCPP_DIR/ggml/src/ggml-common.h" ]]; then
                log_warn "ggml-common.h not found in Metal resources, copying from ggml/src"
                cp "$KOBOLDCPP_DIR/ggml/src/ggml-common.h" "$GGML_METAL_PATH_RESOURCES/ggml-common.h"
            fi
        fi

        # Run KoboldCpp
        # KoboldCpp uses a Python wrapper that loads the compiled shared library
        exec python3 koboldcpp.py $args
    fi
}

main() {
    parse_args "$@"
    check_platform
    check_dependencies

    if [[ "$FORCE_BUILD" == true ]]; then
        log_info "Force rebuild requested..."
        clone_koboldcpp
        build_koboldcpp
        find_binary || {
            log_error "Build completed but binary not found"
            exit 1
        }
    else
        setup_koboldcpp
    fi

    check_model "$DEFAULT_MODEL"
    start_server "$DEFAULT_MODEL"
}

main "$@"
