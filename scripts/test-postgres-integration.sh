#!/usr/bin/env bash
# Edge-case battery for `clickhousectl local postgres`. Runs each test in
# isolation in a temp directory, prints PASS/FAIL, and continues on
# failure so we get a full picture in one run.
#
# Requires:
#   * a working Docker daemon
#   * `jq` on PATH
#
# Usage:
#   scripts/test-postgres-integration.sh [path/to/clickhousectl]
#
# If no argument is given, falls back to $CLICKHOUSECTL or the debug build
# at target/debug/clickhousectl relative to the repo root.
set -u

CTL="${1:-${CLICKHOUSECTL:-}}"
if [[ -z "$CTL" ]]; then
    repo_root=$(cd "$(dirname "$0")/.." && pwd)
    CTL="$repo_root/target/debug/clickhousectl"
fi
if [[ ! -x "$CTL" ]]; then
    echo "clickhousectl binary not found at: $CTL" >&2
    echo "Run 'cargo build -p clickhousectl' first, or pass the path as argument." >&2
    exit 2
fi
if ! command -v jq >/dev/null 2>&1; then
    echo "jq is required" >&2
    exit 2
fi
if ! docker info >/dev/null 2>&1; then
    echo "Docker daemon is not reachable" >&2
    exit 2
fi

PASS=0; FAIL=0
FAILED_TESTS=()

# Make a clean temp dir per test, run the body, then clean up containers
# the test created.
run_case() {
    local name=$1; shift
    local dir; dir=$(mktemp -d -t "pg-edge-$name.XXXX")
    cd "$dir" || { echo "[FAIL] $name: tempdir"; FAIL=$((FAIL+1)); return; }
    if "$@"; then
        echo "[PASS] $name"
        PASS=$((PASS+1))
    else
        echo "[FAIL] $name"
        FAIL=$((FAIL+1))
        FAILED_TESTS+=("$name")
    fi
    # Best-effort cleanup of any containers this test left behind
    docker ps -a --filter "label=clickhousectl.project=$dir" -q 2>/dev/null \
        | xargs -r docker rm -f >/dev/null 2>&1
    cd /
    rm -rf "$dir"
}

# Helper: fail the case
die() { echo "    -> $*"; return 1; }

# ── 1. Reuses existing container after stop/delete-metadata via discovery ──
case_orphan_recovery() {
    "$CTL" local postgres start --name a --version 16-alpine >/dev/null 2>&1 || { die "start"; return 1; }
    local cid_before; cid_before=$(jq -r .container_id .clickhouse/servers/a.json)
    "$CTL" local postgres stop a >/dev/null 2>&1 || { die "stop"; return 1; }
    rm .clickhouse/servers/a.json
    # Recovery should re-populate metadata from the container labels.
    "$CTL" local server list 2>&1 | grep -q "^| a "  || { die "list did not see a"; return 1; }
    [[ -f .clickhouse/servers/a.json ]] || { die "metadata not recovered"; return 1; }
    local cid_after; cid_after=$(jq -r .container_id .clickhouse/servers/a.json)
    [[ "$cid_before" == "$cid_after" ]] || { die "container id changed: $cid_before -> $cid_after"; return 1; }
    "$CTL" local postgres remove a >/dev/null 2>&1 || { die "remove"; return 1; }
}

# ── 2. start with externally-removed container errors with recovery guidance ──
case_externally_removed_container() {
    "$CTL" local postgres start --name b --version 16-alpine >/dev/null 2>&1 || { die "start"; return 1; }
    local cid; cid=$(jq -r .container_id .clickhouse/servers/b.json)
    docker rm -f "$cid" >/dev/null 2>&1 || { die "docker rm failed"; return 1; }
    # Metadata still references the dead id. Should error with explicit
    # recovery guidance, not silently recreate against potentially-corrupt PGDATA.
    local out; out=$("$CTL" local postgres start --name b --version 16-alpine 2>&1) || true
    echo "$out" | grep -q "container is gone" || { die "no recovery message: $out"; return 1; }
    # `local postgres remove` should clean it up.
    "$CTL" local postgres remove b >/dev/null 2>&1 || { die "remove after orphan failed"; return 1; }
    # Now start fresh should work.
    "$CTL" local postgres start --name b --version 16-alpine >/dev/null 2>&1 || { die "fresh start after remove failed"; return 1; }
    "$CTL" local postgres stop b >/dev/null 2>&1
    "$CTL" local postgres remove b >/dev/null 2>&1
}

# ── 3. Two named postgres servers coexist on different ports ──
case_two_concurrent_servers() {
    "$CTL" local postgres start --name c1 --version 16-alpine >/dev/null 2>&1 || { die "start c1"; return 1; }
    "$CTL" local postgres start --name c2 --version 16-alpine >/dev/null 2>&1 || { die "start c2"; return 1; }
    local p1 p2
    p1=$(jq -r .tcp_port .clickhouse/servers/c1.json)
    p2=$(jq -r .tcp_port .clickhouse/servers/c2.json)
    [[ "$p1" != "$p2" ]] || { die "ports collide: $p1 == $p2"; return 1; }
    [[ "$p1" -gt 0 && "$p2" -gt 0 ]] || { die "ports invalid"; return 1; }
    "$CTL" local postgres stop c1 >/dev/null 2>&1
    "$CTL" local postgres stop c2 >/dev/null 2>&1
    "$CTL" local postgres remove c1 >/dev/null 2>&1
    "$CTL" local postgres remove c2 >/dev/null 2>&1
}

# ── 4. Restart with conflicting --version warns and uses stored version ──
case_restart_ignores_changed_version() {
    "$CTL" local postgres start --name d --version 16-alpine >/dev/null 2>&1 || { die "start"; return 1; }
    "$CTL" local postgres stop d >/dev/null 2>&1
    local out; out=$("$CTL" local postgres start --name d --version 17-alpine 2>&1)
    echo "$out" | grep -q "resuming with stored settings" || { die "no resume warning"; return 1; }
    local image; image=$(jq -r .version .clickhouse/servers/d.json)
    [[ "$image" == "postgres:16-alpine" ]] || { die "image changed: $image"; return 1; }
    "$CTL" local postgres stop d >/dev/null 2>&1
    "$CTL" local postgres remove d >/dev/null 2>&1
}

# ── 5. dotenv reflects the actual container password (post-restart) ──
case_dotenv_password_consistency() {
    "$CTL" local postgres start --name e --version 16-alpine >/dev/null 2>&1 || { die "start"; return 1; }
    local pw_meta; pw_meta=$("$CTL" local postgres dotenv --name e 2>/dev/null \
        | grep POSTGRES_PASSWORD= | cut -d= -f2)
    [[ -n "$pw_meta" ]] || { die "no password emitted"; return 1; }
    "$CTL" local postgres stop e >/dev/null 2>&1
    "$CTL" local postgres start --name e >/dev/null 2>&1 || { die "restart"; return 1; }
    local pw_after; pw_after=$("$CTL" local postgres dotenv --name e 2>/dev/null \
        | grep POSTGRES_PASSWORD= | cut -d= -f2)
    [[ "$pw_meta" == "$pw_after" ]] || { die "password drifted: $pw_meta -> $pw_after"; return 1; }
    "$CTL" local postgres stop e >/dev/null 2>&1
    "$CTL" local postgres remove e >/dev/null 2>&1
}

# ── 6. Path-traversal server name rejected ──
case_path_traversal_name() {
    local out; out=$("$CTL" local postgres start --name "../etc" 2>&1) || true
    echo "$out" | grep -qiE "invalid|name" || { die "no rejection: $out"; return 1; }
}

# ── 7. install postgres@latest rejected ──
case_install_rejects_latest() {
    local out; out=$("$CTL" local install postgres@latest 2>&1) || true
    echo "$out" | grep -q "not supported" || { die "no rejection: $out"; return 1; }
}

# ── 8. Cross-engine guard: stopped CH default blocks postgres default ──
case_cross_engine_blocks_postgres() {
    mkdir -p .clickhouse/servers/x/data
    cat > .clickhouse/servers/x.json <<EOF
{"name":"x","pid":99999,"version":"25.12.5.44","http_port":8123,"tcp_port":9000,"started_at":"0","cwd":"$PWD","engine":"clickhouse"}
EOF
    local out; out=$("$CTL" local postgres start --name x 2>&1) || true
    echo "$out" | grep -q "is already in use by a clickhouse server" || { die "no guard: $out"; return 1; }
}

# ── 9. Cross-engine guard: stopped Postgres blocks CH ──
case_cross_engine_blocks_clickhouse() {
    mkdir -p .clickhouse/servers/y/data
    cat > .clickhouse/servers/y.json <<EOF
{"name":"y","pid":0,"version":"postgres:16-alpine","http_port":0,"tcp_port":5432,"started_at":"0","cwd":"$PWD","engine":"postgres","container_id":"deadbeef"}
EOF
    local out; out=$("$CTL" local server start --name y 2>&1) || true
    echo "$out" | grep -q "is already in use by a postgres server" || { die "no guard: $out"; return 1; }
}

# ── 10. local server stop-all leaves Postgres running ──
case_stop_all_isolates_postgres() {
    "$CTL" local postgres start --name p --version 16-alpine >/dev/null 2>&1 || { die "start"; return 1; }
    "$CTL" local server stop-all >/dev/null 2>&1
    "$CTL" local server list 2>&1 | grep -E "^\| p " | grep -q running || { die "postgres got stopped"; return 1; }
    "$CTL" local postgres stop p >/dev/null 2>&1
    "$CTL" local postgres remove p >/dev/null 2>&1
}

# ── 11. --port 0 rejected ──
case_port_zero_rejected() {
    local out; out=$("$CTL" local postgres start --name z --port 0 2>&1) || true
    echo "$out" | grep -q -- "--port 0 is not allowed" || { die "no rejection: $out"; return 1; }
}

# ── 12. Non-TTY query path returns query result ──
case_non_tty_query() {
    "$CTL" local postgres start --name q --version 16-alpine >/dev/null 2>&1 || { die "start"; return 1; }
    # Wait for pg to be query-ready (up to ~5s); the first start already waits
    # for the container, but pg itself takes a moment to accept queries.
    local cid; cid=$(jq -r .container_id .clickhouse/servers/q.json)
    for _ in {1..30}; do
        docker exec "$cid" pg_isready -U postgres >/dev/null 2>&1 && break
        sleep 0.2
    done
    # Hide host psql to force the docker-exec fallback.
    local out; out=$(PATH=/usr/bin:/bin "$CTL" local postgres client --name q --query "select 7 as seven" 2>&1)
    echo "$out" | grep -q "7" || { die "query did not return 7: $out"; return 1; }
    "$CTL" local postgres stop q >/dev/null 2>&1
    "$CTL" local postgres remove q >/dev/null 2>&1
}

# ── 13. dotenv preserves unmanaged vars and replaces in-place ──
case_dotenv_preserves_other_vars() {
    "$CTL" local postgres start --name r --version 16-alpine >/dev/null 2>&1 || { die "start"; return 1; }
    cat > .env <<EOF
DATABASE_URL=postgres://existing
CLICKHOUSE_HOST=ch.example.com
EOF
    "$CTL" local postgres dotenv --name r >/dev/null 2>&1 || { die "dotenv"; return 1; }
    grep -q "DATABASE_URL=postgres://existing" .env || { die "lost DATABASE_URL"; return 1; }
    grep -q "CLICKHOUSE_HOST=ch.example.com" .env || { die "lost CLICKHOUSE_HOST"; return 1; }
    grep -q "POSTGRES_HOST=127.0.0.1" .env || { die "no POSTGRES_HOST"; return 1; }
    "$CTL" local postgres stop r >/dev/null 2>&1
    "$CTL" local postgres remove r >/dev/null 2>&1
}

# ── 14. remove of running postgres is rejected ──
case_remove_running_rejected() {
    "$CTL" local postgres start --name s --version 16-alpine >/dev/null 2>&1 || { die "start"; return 1; }
    local out; out=$("$CTL" local postgres remove s 2>&1) || true
    echo "$out" | grep -qE "already running|running" || { die "no rejection: $out"; return 1; }
    "$CTL" local postgres stop s >/dev/null 2>&1
    "$CTL" local postgres remove s >/dev/null 2>&1
}

# ── 15a. Each supported major (16/17/18) starts and is query-ready ──
case_majors_start_and_serve() {
    local tag fail=0
    for tag in 16 17 18; do
        local n="m$tag"
        if ! "$CTL" local postgres start --name "$n" --version "$tag" >/dev/null 2>&1; then
            die "start postgres:$tag failed"
            fail=1
            continue
        fi
        local cid; cid=$(jq -r .container_id ".clickhouse/servers/$n.json")
        local ready=0
        for _ in {1..50}; do
            if docker exec "$cid" pg_isready -U postgres >/dev/null 2>&1; then
                ready=1
                break
            fi
            sleep 0.2
        done
        if (( ready != 1 )); then
            die "postgres:$tag not query-ready after 10s"
            fail=1
        fi
        "$CTL" local postgres stop "$n" >/dev/null 2>&1
        "$CTL" local postgres remove "$n" >/dev/null 2>&1
    done
    return $fail
}

# ── 16. validate_pg_tag rejects 19 and 14 ──
case_unsupported_majors() {
    local o14 o19
    o14=$("$CTL" local postgres start --name t --version 14-alpine 2>&1) || true
    o19=$("$CTL" local postgres start --name t --version 19 2>&1) || true
    echo "$o14" | grep -q "not supported" || { die "14 not rejected: $o14"; return 1; }
    echo "$o19" | grep -q "not supported" || { die "19 not rejected: $o19"; return 1; }
}

# ── Run all ──
run_case orphan_recovery                case_orphan_recovery
run_case externally_removed_container   case_externally_removed_container
run_case two_concurrent_servers         case_two_concurrent_servers
run_case restart_ignores_changed_version case_restart_ignores_changed_version
run_case dotenv_password_consistency    case_dotenv_password_consistency
run_case path_traversal_name            case_path_traversal_name
run_case install_rejects_latest         case_install_rejects_latest
run_case cross_engine_blocks_postgres   case_cross_engine_blocks_postgres
run_case cross_engine_blocks_clickhouse case_cross_engine_blocks_clickhouse
run_case stop_all_isolates_postgres     case_stop_all_isolates_postgres
run_case port_zero_rejected             case_port_zero_rejected
run_case non_tty_query                  case_non_tty_query
run_case dotenv_preserves_other_vars    case_dotenv_preserves_other_vars
run_case remove_running_rejected        case_remove_running_rejected
run_case majors_start_and_serve         case_majors_start_and_serve
run_case unsupported_majors             case_unsupported_majors

echo
echo "==== $PASS passed, $FAIL failed ===="
if (( FAIL > 0 )); then
    echo "failed: ${FAILED_TESTS[*]}"
    exit 1
fi
