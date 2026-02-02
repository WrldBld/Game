#!/usr/bin/env bash
# KoboldCpp Screen Session Manager
# Starts KoboldCpp in a named screen session with log output
# Supports restart, log viewing, and kill operations with confirmation

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Configuration
SCREEN_NAME="${KOBOLDCPP_SCREEN_NAME:-koboldcpp}"
LOG_DIR="$PROJECT_ROOT/logs"
LOG_FILE="$LOG_DIR/koboldcpp.log"
# Accept MODEL as alias for KOBOLDCPP_MODEL (MODEL takes precedence)
DEFAULT_MODEL="${MODEL:-${KOBOLDCPP_MODEL:-$PROJECT_ROOT/models/Cydonia-24B-v4zg-Q6_K.gguf}}"
HOST="${KOBOLDCPP_HOST:-127.0.0.1}"
PORT="${KOBOLDCPP_PORT:-5001}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_debug() { echo -e "${BLUE}[DEBUG]${NC} $1"; }

# Ensure log directory exists
ensure_log_dir() {
    if [[ ! -d "$LOG_DIR" ]]; then
        mkdir -p "$LOG_DIR"
        log_info "Created log directory: $LOG_DIR"
    fi
}

# Check if screen session exists
screen_exists() {
    screen -ls "$SCREEN_NAME" 2>/dev/null | grep -q "$SCREEN_NAME"
}

# Get screen PID if running
get_screen_pid() {
    screen -ls "$SCREEN_NAME" 2>/dev/null | grep "$SCREEN_NAME" | awk '{print $1}' | cut -d. -f1
}

# Ask for confirmation
confirm() {
    local prompt="$1"
    local response
    echo -n -e "${YELLOW}$prompt [y/N]:${NC} "
    read -r response
    [[ "$response" =~ ^[Yy]$ ]]
}

# Kill screen session
kill_screen() {
    if ! screen_exists; then
        log_warn "Screen session '$SCREEN_NAME' is not running"
        return 1
    fi

    local pid
    pid=$(get_screen_pid)
    log_info "Found screen session: $SCREEN_NAME (PID: $pid)"

    if confirm "Are you sure you want to kill the KoboldCpp screen session?"; then
        log_info "Killing screen session: $SCREEN_NAME"
        screen -S "$SCREEN_NAME" -X quit 2>/dev/null || true
        sleep 1
        if screen_exists; then
            log_warn "Screen session still exists, forcing kill..."
            kill -9 "$pid" 2>/dev/null || true
            sleep 1
        fi
        if screen_exists; then
            log_error "Failed to kill screen session"
            return 1
        else
            log_info "Screen session killed successfully"
            return 0
        fi
    else
        log_info "Kill cancelled"
        return 1
    fi
}

# View logs
view_logs() {
    if [[ ! -f "$LOG_FILE" ]]; then
        log_warn "Log file not found: $LOG_FILE"
        return 1
    fi

    echo ""
    echo "=== KoboldCpp Logs (last 50 lines) ==="
    echo ""
    tail -n 50 "$LOG_FILE"
    echo ""
    echo "=== End of log preview ==="
    echo ""
    echo "Options:"
    echo "  [f]ollow - Follow log output (Ctrl+C to exit)"
    echo "  [a]ll    - View entire log file"
    echo "  [q]uit   - Return to menu"
    echo ""
    echo -n "Choice: "
    local choice
    read -r choice

    case "$choice" in
        f|F)
            echo "Following logs (Ctrl+C to exit)..."
            tail -f "$LOG_FILE"
            ;;
        a|A)
            less "$LOG_FILE"
            ;;
        *)
            return 0
            ;;
    esac
}

# Start KoboldCpp in screen session
start_screen() {
    local model_path="${1:-$DEFAULT_MODEL}"

    if screen_exists; then
        log_error "Screen session '$SCREEN_NAME' already exists"
        log_info "Use the menu to restart or view logs"
        return 1
    fi

    if [[ ! -f "$model_path" ]]; then
        log_error "Model file not found: $model_path"
        log_info "Please provide a valid model path or set KOBOLDCPP_MODEL"
        return 1
    fi

    ensure_log_dir

    log_info "Starting KoboldCpp in screen session: $SCREEN_NAME"
    log_info "Model: $model_path"
    log_info "Log file: $LOG_FILE"

    # Build command arguments
    local args="--model $model_path --host $HOST --port $PORT"

    # Check if using Nix-installed KoboldCpp
    local koboldcpp_cmd
    if command -v koboldcpp >/dev/null 2>&1; then
        koboldcpp_cmd="koboldcpp"
        log_info "Using KoboldCpp from Nix"
    else
        local koboldcpp_dir="${KOBOLDCPP_DIR:-$PROJECT_ROOT/tools/koboldcpp}"
        if [[ ! -f "$koboldcpp_dir/koboldcpp.py" ]]; then
            log_error "KoboldCpp not found. Please run: ./scripts/serve-koboldcpp.sh --build"
            return 1
        fi
        koboldcpp_cmd="python3 $koboldcpp_dir/koboldcpp.py"
        log_info "Using KoboldCpp from source: $koboldcpp_dir"
        # Set Metal shader path for source builds
        export GGML_METAL_PATH_RESOURCES="$koboldcpp_dir/ggml/src/ggml-metal"

        # Some versions of KoboldCpp expect ggml-metal-merged.metal with inlined headers
        # Generate it if it doesn't exist
        if [[ ! -f "$GGML_METAL_PATH_RESOURCES/ggml-metal-merged.metal" ]]; then
            local metal_src="$koboldcpp_dir/ggml/src/ggml-metal/ggml-metal.metal"
            local common_src="$koboldcpp_dir/ggml/src/ggml-common.h"
            local impl_src="$koboldcpp_dir/ggml/src/ggml-metal/ggml-metal-impl.h"

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
            if [[ -f "$koboldcpp_dir/ggml/src/ggml-common.h" ]]; then
                log_warn "ggml-common.h not found in Metal resources, copying from ggml/src"
                cp "$koboldcpp_dir/ggml/src/ggml-common.h" "$GGML_METAL_PATH_RESOURCES/ggml-common.h"
            fi
        fi
    fi

    # Start screen session with logging
    screen -dmS "$SCREEN_NAME" bash -c "
        echo \"Starting KoboldCpp at \$(date)\" | tee -a '$LOG_FILE'
        echo \"Command: $koboldcpp_cmd $args\" | tee -a '$LOG_FILE'
        echo \"\" | tee -a '$LOG_FILE'
        exec $koboldcpp_cmd $args 2>&1 | tee -a '$LOG_FILE'
    "

    sleep 2

    if screen_exists; then
        log_info "Screen session started successfully"
        log_info ""
        log_info "To attach to the session: screen -r $SCREEN_NAME"
        log_info "To detach from session: Ctrl+A, then D"
        log_info "API endpoint: http://$HOST:$PORT/v1/chat/completions"
        return 0
    else
        log_error "Failed to start screen session"
        return 1
    fi
}

# Restart screen session
restart_screen() {
    local model_path="${1:-$DEFAULT_MODEL}"

    if screen_exists; then
        if confirm "Are you sure you want to restart the KoboldCpp screen session?"; then
            log_info "Restarting KoboldCpp screen session..."
            kill_screen
            sleep 2
            start_screen "$model_path"
        else
            log_info "Restart cancelled"
        fi
    else
        log_info "No existing session found, starting fresh..."
        start_screen "$model_path"
    fi
}

# Show menu for existing session
show_menu() {
    local model_path="${1:-$DEFAULT_MODEL}"

    echo ""
    echo "========================================"
    echo "  KoboldCpp Screen Session Manager"
    echo "========================================"
    echo ""
    echo "Screen session '$SCREEN_NAME' is already running."
    echo ""
    echo "Options:"
    echo "  [r]estart  - Kill and restart the session"
    echo "  [l]ogs     - View log output"
    echo "  [k]ill     - Kill the session"
    echo "  [a]ttach   - Attach to the screen session"
    echo "  [q]uit     - Exit this menu"
    echo ""
    echo -n "Choice: "
    local choice
    read -r choice

    case "$choice" in
        r|R)
            restart_screen "$model_path"
            ;;
        l|L)
            view_logs
            show_menu "$model_path"
            ;;
        k|K)
            kill_screen
            ;;
        a|A)
            log_info "Attaching to screen session (Ctrl+A, D to detach)..."
            screen -r "$SCREEN_NAME"
            ;;
        *)
            log_info "Exiting menu"
            ;;
    esac
}

# Print usage
usage() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS] [MODEL_PATH]

Start KoboldCpp in a screen session for background operation.

Arguments:
    MODEL_PATH           Path to GGUF model file
                         Default: $DEFAULT_MODEL

Options:
    -n, --name NAME      Screen session name (default: $SCREEN_NAME)
    -p, --port PORT      Port to listen on (default: $PORT)
    -H, --host HOST      Host to bind to (default: $HOST)
    -k, --kill           Kill the screen session
    -r, --restart        Restart the screen session
    -l, --logs           View logs
    -a, --attach         Attach to existing session
    -h, --help           Show this help message

Environment Variables:
    MODEL                   Default model path (alias for KOBOLDCPP_MODEL)
    KOBOLDCPP_MODEL         Default model path
    KOBOLDCPP_SCREEN_NAME   Screen session name (default: koboldcpp)
    KOBOLDCPP_HOST          Host to bind to
    KOBOLDCPP_PORT          Port to listen on

Examples:
    # Start with default model
    $(basename "$0")

    # Start with specific model
    $(basename "$0") /path/to/model.gguf

    # Start with custom screen name
    KOBOLDCPP_SCREEN_NAME=llm1 $(basename "$0")

    # Start with model via environment variable
    MODEL=/path/to/model.gguf $(basename "$0")

    # Kill existing session
    $(basename "$0") --kill

    # Restart session with new model
    $(basename "$0") --restart /path/to/new-model.gguf

    # View logs
    $(basename "$0") --logs

Log File:
    $LOG_FILE

Screen Commands:
    Attach:    screen -r $SCREEN_NAME
    Detach:    Ctrl+A, then D
    Kill:      screen -S $SCREEN_NAME -X quit
EOF
}

# Parse arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -n|--name)
                SCREEN_NAME="$2"
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
            -k|--kill)
                kill_screen
                exit 0
                ;;
            -r|--restart)
                restart_screen "${2:-$DEFAULT_MODEL}"
                exit 0
                ;;
            -l|--logs)
                view_logs
                exit 0
                ;;
            -a|--attach)
                if screen_exists; then
                    screen -r "$SCREEN_NAME"
                else
                    log_error "Screen session '$SCREEN_NAME' not found"
                    exit 1
                fi
                exit 0
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
                DEFAULT_MODEL="$1"
                shift
                ;;
        esac
    done
}

main() {
    parse_args "$@"

    # Check if screen is available
    if ! command -v screen >/dev/null 2>&1; then
        log_error "GNU Screen is not installed"
        log_info "Install with: nix develop (includes screen)"
        exit 1
    fi

    if screen_exists; then
        show_menu "$DEFAULT_MODEL"
    else
        start_screen "$DEFAULT_MODEL"
    fi
}

main "$@"
