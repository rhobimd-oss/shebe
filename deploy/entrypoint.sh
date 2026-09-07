#!/bin/sh
# Entrypoint for shebe-dev-musl container
# Provides functions to run commands with/without sccache
set -eu

#------------------------------------------------------------------------------
# sccache lifecycle
#------------------------------------------------------------------------------

sccache_start() {
    echo "[sccache] Starting server..."

    # Start server in background (avoids timeout with slow S3 init)
    SCCACHE_START_SERVER=1 SCCACHE_NO_DAEMON=1 sccache > /tmp/sccache.log 2>&1 &
    SCCACHE_PID=$!

    # Wait for server to be ready (up to 10 seconds)
    for i in 1 2 3 4 5 6 7 8 9 10; do
        if sccache --show-stats > /dev/null 2>&1; then
            echo "[sccache] Server running (PID: $SCCACHE_PID)"
            sccache --show-stats | grep -E "^Cache location"
            return 0
        fi
        sleep 1
    done

    echo "[sccache] ERROR: Server failed to start"
    cat /tmp/sccache.log
    return 1
}

sccache_stop() {
    echo "[sccache] Stopping server..."
    sccache --stop-server 2>/dev/null || true
}

sccache_stats() {
    echo "[sccache] Stats:"
    sccache --show-stats | grep -E "^(Compile requests|Cache hits|Cache misses)"
}

# The first compile request makes the server hash the rustc toolchain.
# On a cold container this hash takes 45 to 60 seconds, and the sccache
# client aborts the request at 60 seconds. Warm the digest here, where
# no client timeout applies, so the cargo probes get a fast response.
sccache_warm() {
    rustc_path="$(rustup which rustc 2>/dev/null || command -v rustc)"
    echo "[sccache] Warming compiler digest for $rustc_path ..."
    for _attempt in 1 2 3; do
        if sccache "$rustc_path" -vV > /dev/null 2>&1; then
            echo "[sccache] Compiler digest ready"
            return 0
        fi
        echo "[sccache] Warm attempt $_attempt failed, retrying..."
    done
    echo "[sccache] WARNING: digest warm-up failed, builds may be slow"
    return 0
}

#------------------------------------------------------------------------------
# Run commands WITH sccache
#------------------------------------------------------------------------------

with_sccache() {
    sccache_start || return 1
    trap sccache_stop EXIT
    sccache_warm
    echo "[sccache] Running: $*"
    "$@"
    local exit_code=$?
    sccache_stats
    return $exit_code
}

#------------------------------------------------------------------------------
# Run commands WITHOUT sccache
#------------------------------------------------------------------------------

without_sccache() {
    echo "[no-sccache] Running: $*"
    RUSTC_WRAPPER="" "$@"
}

#------------------------------------------------------------------------------
# Main
#------------------------------------------------------------------------------

case "${1:-}" in
    with-sccache)
        shift
        with_sccache "$@"
        ;;
    without-sccache)
        shift
        without_sccache "$@"
        ;;
    *)
        # Default: run command directly (sccache via RUSTC_WRAPPER env var)
        exec "$@"
        ;;
esac
