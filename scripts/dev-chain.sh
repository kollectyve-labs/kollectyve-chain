#!/usr/bin/env bash
# Bring up a local Kollectyve dev chain plus the Ethereum JSON-RPC proxy, so that
# MetaMask / Remix / Hardhat can talk to pallet-revive.
#
#   substrate RPC : ws://127.0.0.1:9944
#   ethereum RPC  : http://127.0.0.1:8545  (chain id 28000)
#
# Ctrl-C stops both processes.
set -euo pipefail

CHAIN_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROFILE="${PROFILE:-release}"
BIN_DIR="$CHAIN_DIR/target/$PROFILE"
NODE_BIN="${NODE_BIN:-$BIN_DIR/kollectyve-node}"
ETH_RPC_BIN="${ETH_RPC_BIN:-$BIN_DIR/eth-rpc}"
LOG_DIR="${LOG_DIR:-$CHAIN_DIR/target/dev-chain-logs}"

for bin in "$NODE_BIN" "$ETH_RPC_BIN"; do
	if [[ ! -x "$bin" ]]; then
		echo "missing $bin — build first:" >&2
		echo "  cargo build --release --workspace" >&2
		exit 1
	fi
done

mkdir -p "$LOG_DIR"
NODE_LOG="$LOG_DIR/node.log"
ETH_LOG="$LOG_DIR/eth-rpc.log"

pids=()
cleanup() {
	trap - INT TERM EXIT
	echo
	echo "stopping dev chain…"
	for pid in "${pids[@]:-}"; do
		[[ -n "$pid" ]] && kill "$pid" 2>/dev/null || true
	done
	wait 2>/dev/null || true
}
trap cleanup INT TERM EXIT

# `--state-pruning archive` is required for eth-rpc's default `--eth-pruning archive`,
# which replays historical blocks to build the receipt index.
echo "starting kollectyve-node  (log: $NODE_LOG)"
"$NODE_BIN" \
	--dev \
	--rpc-cors=all \
	--state-pruning=archive \
	--blocks-pruning=archive \
	>"$NODE_LOG" 2>&1 &
pids+=($!)

# Wait for the substrate RPC port before starting the proxy.
for _ in $(seq 1 60); do
	if curl -s -o /dev/null -H 'content-type: application/json' \
		-d '{"jsonrpc":"2.0","id":1,"method":"system_chain","params":[]}' \
		http://127.0.0.1:9944; then
		break
	fi
	sleep 1
done

echo "starting eth-rpc          (log: $ETH_LOG)"
"$ETH_RPC_BIN" \
	--dev \
	--node-rpc-url=ws://127.0.0.1:9944 \
	--rpc-port=8545 \
	--rpc-cors=all \
	>"$ETH_LOG" 2>&1 &
pids+=($!)

cat <<EOF

  substrate RPC : ws://127.0.0.1:9944   (Polkadot-JS Apps)
  ethereum RPC  : http://127.0.0.1:8545 (MetaMask / Remix / Hardhat), chain id 28000

  logs: tail -f $NODE_LOG $ETH_LOG
  Ctrl-C to stop.

EOF

wait
