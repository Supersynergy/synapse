#!/usr/bin/env bash
# remote-mcp.sh — expose synapse-mcp (stdio) and/or cua-driver (stdio) as
# Streamable HTTP + SSE endpoints via mcp-proxy, then optionally tunnel via
# cloudflared or tailscale serve.
#
# Usage:
#   remote-mcp.sh synapse   [port]   # default 8765
#   remote-mcp.sh cua       [port]   # default 8766
#   remote-mcp.sh all       [port]   # both, ports 8765/8766
#   remote-mcp.sh tunnel synapse    # cloudflared tunnel on top of synapse
#   remote-mcp.sh stop                # kill all spawned processes
#
# Requirements:
#   - mcp-proxy (uv tool install mcp-proxy)
#   - synapse-mcp binary (cargo build -p synapse-mcp --release)
#   - cua-driver binary (~/.local/bin/cua-driver)
#   - cloudflared (optional, for public tunnel)
#   - tailscale  (optional, for mesh expose)

set -euo pipefail

SYNAPSE_BIN="${SYNAPSE_BIN:-/Users/master/BASE/projects/synapse-memory/target/release/synapse-mcp}"
SYNAPSE_SOCK="${SYNAPSE_SOCK:-/tmp/synapse.sock}"
CUA_BIN="${CUA_BIN:-$HOME/.local/bin/cua-driver}"
PID_DIR="${PID_DIR:-/tmp/remote-mcp-pids}"
mkdir -p "$PID_DIR"

log() { printf '[remote-mcp] %s\n' "$*" >&2; }

start_synapse() {
  local port="${1:-8765}"
  if [[ ! -x "$SYNAPSE_BIN" ]]; then
    log "ERROR: $SYNAPSE_BIN not found. Run: cargo build -p synapse-mcp --release"
    return 1
  fi
  if [[ ! -S "$SYNAPSE_SOCK" ]]; then
    log "WARN: $SYNAPSE_SOCK not found — start synapsed first: synapsed --sock $SYNAPSE_SOCK --file ~/.synapse/brain.db"
  fi
  log "starting synapse-mcp proxy on :$port (stdio -> streamable HTTP + SSE)"
  mcp-proxy --port "$port" --host 127.0.0.1 -- "$SYNAPSE_BIN" -s "$SYNAPSE_SOCK" \
    >"$PID_DIR/synapse.log" 2>&1 &
  echo $! > "$PID_DIR/synapse.pid"
  log "  pid=$(cat "$PID_DIR/synapse.pid")  log=$PID_DIR/synapse.log"
  log "  endpoints: http://127.0.0.1:$port/sse  http://127.0.0.1:$port/mcp"
}

start_cua() {
  local port="${1:-8766}"
  if [[ ! -x "$CUA_BIN" ]]; then
    log "ERROR: $CUA_BIN not found."
    return 1
  fi
  log "starting cua-driver proxy on :$port (stdio -> streamable HTTP + SSE)"
  mcp-proxy --port "$port" --host 127.0.0.1 -- "$CUA_BIN" mcp \
    >"$PID_DIR/cua.log" 2>&1 &
  echo $! > "$PID_DIR/cua.pid"
  log "  pid=$(cat "$PID_DIR/cua.pid")  log=$PID_DIR/cua.log"
  log "  endpoints: http://127.0.0.1:$port/sse  http://127.0.0.1:$port/mcp"
}

start_tunnel() {
  local target="$1"
  local port
  case "$target" in
    synapse) port=8765 ;;
    cua)     port=8766 ;;
    *) log "tunnel: unknown target $target"; return 1 ;;
  esac
  if ! command -v cloudflared >/dev/null 2>&1; then
    log "ERROR: cloudflared not installed."
    return 1
  fi
  log "starting cloudflared tunnel for $target (localhost:$port)"
  cloudflared tunnel --url "http://127.0.0.1:$port" \
    >"$PID_DIR/tunnel-$target.log" 2>&1 &
  echo $! > "$PID_DIR/tunnel-$target.pid"
  log "  pid=$(cat "$PID_DIR/tunnel-$target.pid")  log=$PID_DIR/tunnel-$target.log"
  log "  public URL printed in log file above (trycloudflare.com)"
}

stop_all() {
  for f in "$PID_DIR"/*.pid; do
    [[ -f "$f" ]] || continue
    local pid; pid=$(cat "$f")
    if kill "$pid" 2>/dev/null; then
      log "stopped pid=$pid ($f)"
    fi
    rm -f "$f"
  done
}

case "${1:-}" in
  synapse) start_synapse "${2:-8765}" ;;
  cua)     start_cua     "${2:-8766}" ;;
  all)     start_synapse "${2:-8765}"; start_cua 8766 ;;
  tunnel)  start_tunnel  "${2:-synapse}" ;;
  stop)    stop_all ;;
  *) echo "usage: $0 {synapse|cua|all|tunnel <target>|stop} [port]"; exit 1 ;;
esac
