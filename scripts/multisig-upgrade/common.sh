#!/usr/bin/env bash
# Shared helpers for the multisig upgrade scripts.
# Source this file from every step script; do not execute it directly.
#
#   source "$(dirname "$0")/common.sh"
#
# Configuration is taken from env vars (see defaults below) so each step can
# be run independently by different signers. Override on the command line or
# in a .env file you `source` before running a script.

set -euo pipefail

# ---------- config defaults ---------------------------------------------------

: "${MULTISIG_ID:=CCGDOYLHZCZ3CFG5QLRZQTYYCIDHKCSGYKZPUAAB5ZDONSFW6T4JCOSY}"
: "${NETWORK:=mainnet}"
: "${WASM_TARGET:=wasm32v1-none}"
: "${REPO_ROOT:=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
: "${WASM_PATH:=$REPO_ROOT/target/$WASM_TARGET/release/soroban_multisig.wasm}"

# Pinned hash of the wasm produced by `cargo build --target wasm32v1-none --release`
# from this branch. Every advisor MUST independently recompute and compare to
# this value before signing the upgrade proposal. If the values diverge the
# upload is suspect.
: "${EXPECTED_WASM_SHA256:=75cf3a582b622db2c77d54a3c3e85c9b18c857e37daf2bab76cc81f2931771b0}"

# ---------- ANSI colors (tty only) -------------------------------------------

if [[ -t 1 ]]; then
  C_RESET=$'\033[0m'
  C_BOLD=$'\033[1m'
  C_DIM=$'\033[2m'
  C_RED=$'\033[31m'
  C_GREEN=$'\033[32m'
  C_YELLOW=$'\033[33m'
  C_BLUE=$'\033[34m'
  C_CYAN=$'\033[36m'
else
  C_RESET=''; C_BOLD=''; C_DIM=''; C_RED=''; C_GREEN=''; C_YELLOW=''; C_BLUE=''; C_CYAN=''
fi

log()    { printf '%s[+]%s %s\n' "$C_BLUE"   "$C_RESET" "$*"; }
ok()     { printf '%s[✓]%s %s\n' "$C_GREEN"  "$C_RESET" "$*"; }
warn()   { printf '%s[!]%s %s\n' "$C_YELLOW" "$C_RESET" "$*" >&2; }
err()    { printf '%s[✗]%s %s\n' "$C_RED"    "$C_RESET" "$*" >&2; }
hdr()    { printf '\n%s%s== %s ==%s\n' "$C_BOLD" "$C_CYAN" "$*" "$C_RESET"; }
kv()     { printf '    %s%-22s%s %s\n' "$C_DIM" "$1" "$C_RESET" "$2"; }
die()    { err "$*"; exit 1; }

# ---------- preflight checks --------------------------------------------------

require_cmd() {
  local cmd=$1
  command -v "$cmd" >/dev/null 2>&1 || die "Required command not found: $cmd"
}

require_cmd stellar
require_cmd sha256sum
require_cmd jq

# ---------- argument helpers --------------------------------------------------

# Verify a strkey looks like a Stellar G-account address.
is_g_address() {
  local addr=$1
  [[ ${#addr} -eq 56 && $addr =~ ^G[A-Z2-7]{55}$ ]]
}

require_g_address() {
  local name=$1 value=${2:-}
  [[ -n $value ]] || die "$name is required (G-address)"
  is_g_address "$value" || die "$name does not look like a G-address: $value"
}

# Confirm with the user before doing something destructive. Auto-confirm when
# YES=1 in the environment (for non-interactive runs). On non-tty stdin, refuse
# rather than silently proceeding.
confirm() {
  local prompt=${1:-"Proceed?"}
  if [[ ${YES:-0} == 1 ]]; then
    log "$prompt (auto-confirmed via YES=1)"
    return 0
  fi
  if [[ ! -t 0 ]]; then
    die "$prompt — stdin is not a terminal; set YES=1 to auto-confirm"
  fi
  local reply
  read -r -p "$(printf '%s%s%s [y/N] ' "$C_YELLOW" "$prompt" "$C_RESET")" reply
  [[ $reply =~ ^[Yy]$ ]] || die "Aborted by user"
}

# ---------- key resolution ----------------------------------------------------

# Resolve a stellar CLI key alias OR G-address to its G-address. Accepts either
# form so scripts work whether the signer pasted in a strkey or named a key.
resolve_g_address() {
  local key=$1
  if is_g_address "$key"; then
    printf '%s\n' "$key"
    return 0
  fi
  stellar keys public-key "$key" 2>/dev/null \
    || die "Cannot resolve '$key' — not a G-address and not a known stellar CLI key alias"
}

# ---------- multisig CLI wrapper ---------------------------------------------

# Read-only contract invocation. Uses a dummy source account because Soroban
# requires one for fee simulation, but never broadcasts.
multisig_query() {
  local fn=$1; shift
  local source=${MULTISIG_QUERY_SOURCE:-}
  if [[ -z $source ]]; then
    # Fall back to the first available local key. Any G-account works for queries.
    source=$(stellar keys ls 2>/dev/null | head -1)
    [[ -n $source ]] || die "Cannot pick a query-source account; set MULTISIG_QUERY_SOURCE"
  fi
  stellar contract invoke \
    --network "$NETWORK" \
    --id "$MULTISIG_ID" \
    --source-account "$source" \
    --send=no \
    -- "$fn" "$@"
}

# Write invocation that simulates by default and only broadcasts when
# BROADCAST=1. Every step script flips this based on its own --broadcast flag.
multisig_invoke() {
  local source=$1; shift
  local fn=$1; shift
  local send_flag=(--send=no)
  if [[ ${BROADCAST:-0} == 1 ]]; then
    send_flag=()
  fi
  stellar contract invoke \
    --network "$NETWORK" \
    --id "$MULTISIG_ID" \
    --source-account "$source" \
    "${send_flag[@]}" \
    -- "$fn" "$@"
}

# ---------- session banner ---------------------------------------------------

print_session_header() {
  hdr "${1:-Multisig upgrade step}"
  kv "Network"          "$NETWORK"
  kv "Multisig contract" "$MULTISIG_ID"
  kv "Wasm path"        "$WASM_PATH"
  kv "Expected sha256"  "$EXPECTED_WASM_SHA256"
}
