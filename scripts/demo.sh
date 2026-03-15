#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

HARNESS="${NEXODE_DEMO_HARNESS:-}"
MODEL=""
API_ENV=""

command_exists() {
  command -v "$1" >/dev/null 2>&1
}

if [[ -z "${HARNESS}" ]]; then
  if command_exists claude && [[ -n "${ANTHROPIC_API_KEY:-}" ]]; then
    HARNESS="claude-code"
  elif command_exists codex && [[ -n "${OPENAI_API_KEY:-}" ]]; then
    HARNESS="codex-cli"
  else
    echo "No live harness available. Set NEXODE_DEMO_HARNESS and the matching API key."
    exit 1
  fi
fi

case "${HARNESS}" in
  claude-code)
    MODEL="${NEXODE_DEMO_MODEL:-claude-sonnet-4-5}"
    API_ENV="ANTHROPIC_API_KEY"
    ;;
  codex-cli)
    MODEL="${NEXODE_DEMO_MODEL:-gpt-4.1}"
    API_ENV="OPENAI_API_KEY"
    ;;
  *)
    echo "Unsupported NEXODE_DEMO_HARNESS: ${HARNESS}"
    exit 1
    ;;
esac

if [[ -z "${!API_ENV:-}" ]]; then
  echo "Missing required environment variable: ${API_ENV}"
  exit 1
fi

REPO_DIR="${TMP_DIR}/repo"
SESSION_DIR="${TMP_DIR}/session"
SESSION_FILE="${SESSION_DIR}/session.yaml"
ADDR="${NEXODE_DEMO_ADDR:-http://127.0.0.1:50051}"

mkdir -p "${REPO_DIR}" "${SESSION_DIR}"

git -C "${TMP_DIR}" init -b main "${REPO_DIR}" >/dev/null
git -C "${REPO_DIR}" config user.email "demo@example.com"
git -C "${REPO_DIR}" config user.name "Nexode Demo"
printf '# Nexode Demo\n' > "${REPO_DIR}/README.md"
git -C "${REPO_DIR}" add .
git -C "${REPO_DIR}" commit -m "initial" >/dev/null

cat > "${SESSION_FILE}" <<EOF
version: "2.0"
session:
  name: "demo"
defaults:
  model: "${MODEL}"
  mode: "plan"
  timeout_minutes: 2
projects:
  - id: "project-1"
    repo: "../repo"
    display_name: "Demo Project"
    slots:
      - id: "slot-a"
        harness: "${HARNESS}"
        task: "Add a hello() function to hello.rs that returns the string 'Hello from Nexode'. Commit the change."
EOF

pushd "${ROOT_DIR}" >/dev/null

echo "Starting daemon with ${HARNESS} (${MODEL})"
cargo run -p nexode-daemon -- "${SESSION_FILE}" --listen "${ADDR#http://}" &
DAEMON_PID=$!
trap 'kill ${DAEMON_PID} >/dev/null 2>&1 || true; rm -rf "${TMP_DIR}"' EXIT

sleep 3

echo "Initial status:"
cargo run -p nexode-ctl -- --addr "${ADDR}" status

echo "Watching briefly for REVIEW..."
for _ in $(seq 1 30); do
  STATUS_OUTPUT="$(cargo run -p nexode-ctl -- --addr "${ADDR}" status)"
  echo "${STATUS_OUTPUT}"
  if grep -q "status review" <<<"${STATUS_OUTPUT}"; then
    break
  fi
  sleep 2
done

echo "Queueing merge:"
cargo run -p nexode-ctl -- --addr "${ADDR}" dispatch move-task slot-a merge-queue || true

echo "Final status:"
cargo run -p nexode-ctl -- --addr "${ADDR}" status

echo "Repo contents:"
find "${REPO_DIR}" -maxdepth 2 -type f | sort

popd >/dev/null
