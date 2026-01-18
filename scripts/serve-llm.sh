#!/usr/bin/env bash
# MLX LLM Server for macOS Apple Silicon
# Serves local LLMs with an OpenAI-compatible API using Apple's MLX framework

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
VENV_DIR="${PROJECT_ROOT}/.venv-mlx"

# Default configuration
MODEL="${MLX_MODEL:-mlx-community/Llama-3.1-8B-Instruct}"
PORT="${MLX_PORT:-8000}"
HOST="${MLX_HOST:-127.0.0.1}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Check we're on macOS with Apple Silicon
check_platform() {
    if [[ "$(uname)" != "Darwin" ]]; then
        log_error "This script requires macOS. MLX only runs on Apple Silicon."
        exit 1
    fi

    if [[ "$(uname -m)" != "arm64" ]]; then
        log_error "This script requires Apple Silicon (arm64). MLX does not support Intel Macs."
        exit 1
    fi

    log_info "Platform check passed: macOS on Apple Silicon"
}

# Setup Python virtual environment with MLX packages
setup_venv() {
    if [[ ! -d "$VENV_DIR" ]]; then
        log_info "Creating Python virtual environment at $VENV_DIR"
        python3 -m venv "$VENV_DIR"
    fi

    # Activate venv
    source "${VENV_DIR}/bin/activate"

    # Check if mlx-lm is installed
    if ! python3 -c "import mlx_lm" 2>/dev/null; then
        log_info "Installing MLX packages..."
        pip install --upgrade pip
        pip install mlx mlx-lm
        log_info "MLX packages installed successfully"
    else
        log_info "MLX packages already installed"
    fi
}

# Print usage
usage() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS]

Serve LLMs locally using MLX with an OpenAI-compatible API.

Options:
    -m, --model MODEL    Model to serve (default: $MODEL)
    -p, --port PORT      Port to listen on (default: $PORT)
    -H, --host HOST      Host to bind to (default: $HOST)
    -h, --help           Show this help message

Environment Variables:
    MLX_MODEL    Model to serve
    MLX_PORT     Port to listen on
    MLX_HOST     Host to bind to

Examples:
    $(basename "$0")
    $(basename "$0") --model mlx-community/Mistral-7B-Instruct-v0.3
    $(basename "$0") --port 8080
    MLX_MODEL=mlx-community/Qwen2.5-7B-Instruct $(basename "$0")

Popular MLX Models:
    mlx-community/Llama-3.1-8B-Instruct
    mlx-community/Llama-3.2-3B-Instruct
    mlx-community/Mistral-7B-Instruct-v0.3
    mlx-community/Qwen2.5-7B-Instruct
    mlx-community/gemma-2-9b-it

The server provides an OpenAI-compatible API at:
    http://HOST:PORT/v1/chat/completions
    http://HOST:PORT/v1/completions
    http://HOST:PORT/v1/models
EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -m|--model)
                MODEL="$2"
                shift 2
                ;;
            -p|--port)
                PORT="$2"
                shift 2
                ;;
            -H|--host)
                HOST="$2"
                shift 2
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done
}

# Start the MLX server
start_server() {
    log_info "Starting MLX LLM server..."
    log_info "Model: $MODEL"
    log_info "Endpoint: http://${HOST}:${PORT}"
    echo ""

    # mlx-lm provides the server module
    python3 -m mlx_lm.server \
        --model "$MODEL" \
        --host "$HOST" \
        --port "$PORT"
}

main() {
    parse_args "$@"
    check_platform
    setup_venv
    start_server
}

main "$@"
