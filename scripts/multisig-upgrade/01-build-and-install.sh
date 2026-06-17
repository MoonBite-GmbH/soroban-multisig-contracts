#!/usr/bin/env bash
#
# Step 1: build the new multisig wasm, verify its sha256 matches the pinned
# expected hash, and (optionally) upload it to mainnet so it gets a contract
# hash that future proposals can reference.
#
# Run this ONCE on the machine that submits the install transaction. Every
# other advisor should run 02-verify-hash.sh on their own machine to confirm
# the binary the proposal points at is exactly the one built from this branch.
#
# Usage:
#   ./01-build-and-install.sh                    # build + checksum only
#   ./01-build-and-install.sh --install KEY      # build + upload via signer KEY
#   ./01-build-and-install.sh --install KEY --broadcast
#
# KEY is either a stellar CLI key alias (recommended) or a G-address whose
# secret is loaded into the stellar CLI.

set -euo pipefail
source "$(dirname "$0")/common.sh"

INSTALL_KEY=""
BROADCAST=0

while [[ $# -gt 0 ]]; do
  case $1 in
    --install)    INSTALL_KEY=${2:?--install requires a key alias}; shift 2 ;;
    --broadcast)  BROADCAST=1; shift ;;
    -h|--help)
      sed -n '/^# Step 1/,/^set/p' "$0" | sed 's/^# \{0,1\}//' | head -n -1
      exit 0 ;;
    *) die "Unknown argument: $1" ;;
  esac
done

print_session_header "Step 1 — build & install new wasm"

# ---- Build -----------------------------------------------------------------
hdr "Building wasm"
( cd "$REPO_ROOT" && cargo build --target "$WASM_TARGET" --release )

[[ -f $WASM_PATH ]] || die "Build succeeded but $WASM_PATH is missing"

local_hash=$(sha256sum "$WASM_PATH" | awk '{print $1}')
kv "Built wasm"        "$WASM_PATH ($(stat -c%s "$WASM_PATH") bytes)"
kv "Local sha256"      "$local_hash"
kv "Expected sha256"   "$EXPECTED_WASM_SHA256"

if [[ "$local_hash" != "$EXPECTED_WASM_SHA256" ]]; then
  warn "Local hash differs from the pinned EXPECTED_WASM_SHA256."
  warn "This is fine ONLY if you intentionally changed the source. Otherwise"
  warn "your toolchain or dependency lockfile diverges from what was reviewed."
  confirm "Continue anyway?"
else
  ok "Hash matches the pinned EXPECTED_WASM_SHA256"
fi

# ---- Optional install ------------------------------------------------------
if [[ -z $INSTALL_KEY ]]; then
  ok "Done. Re-run with --install <signer-key> to upload the wasm to $NETWORK."
  exit 0
fi

signer_g=$(resolve_g_address "$INSTALL_KEY")
hdr "Installing wasm on $NETWORK"
kv "Signer alias/addr"  "$INSTALL_KEY"
kv "Resolves to"        "$signer_g"

if [[ $BROADCAST != 1 ]]; then
  warn "--broadcast not passed; this would only simulate the upload."
  warn "Re-run with --broadcast to actually submit the transaction."
  exit 0
fi

confirm "Upload wasm to $NETWORK with signer $signer_g?"

set +e
install_output=$(stellar contract install \
  --network "$NETWORK" \
  --source-account "$INSTALL_KEY" \
  --wasm "$WASM_PATH" 2>&1)
install_status=$?
set -e

printf '%s\n' "$install_output"
[[ $install_status -eq 0 ]] || die "stellar contract install failed (exit $install_status)"

# The CLI prints the wasm hash on the last line of stdout.
installed_hash=$(printf '%s\n' "$install_output" \
  | grep -oE '^[0-9a-f]{64}$' | tail -1)

if [[ -z $installed_hash ]]; then
  warn "Could not extract the installed wasm hash from the CLI output."
  warn "Inspect the output above manually."
  exit 1
fi

ok "Wasm installed on $NETWORK"
kv "On-chain hash"     "$installed_hash"

if [[ "$installed_hash" != "$local_hash" ]]; then
  err "On-chain hash differs from locally computed hash!"
  err "  local:    $local_hash"
  err "  on-chain: $installed_hash"
  err "Do NOT use this hash in a proposal until this is investigated."
  exit 2
fi

ok "On-chain hash matches local build. Use this hash in step 03:"
printf '\n    NEW_WASM_HASH=%s\n\n' "$installed_hash"
