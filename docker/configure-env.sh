#!/bin/sh
# Configure runtime environment variables for the Player web app
# This script runs at container startup to inject environment variables

set -e

# Default WebSocket URL if not provided
ENGINE_WS_URL="${ENGINE_WS_URL:-ws://localhost:3000/ws}"

# Find and update config in the HTML/JS files
# The WASM app should look for window.WRLDBLDR_CONFIG or similar
if [ -f /usr/share/nginx/html/index.html ]; then
    # Inject config script into index.html
    sed -i "s|</head>|<script>window.WRLDBLDR_CONFIG = { engineWsUrl: '${ENGINE_WS_URL}' };</script></head>|" /usr/share/nginx/html/index.html
fi

echo "Configured ENGINE_WS_URL=${ENGINE_WS_URL}"
