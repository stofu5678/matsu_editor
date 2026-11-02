# Settings and helpers shared by mac-runner and build-golden (ADR-0027). Sourced, not run. macOS's bash 3.2.

REPOSITORY=stofu5678/matsu_editor
REPOSITORY_ID=1412713500
# The runner's labels. The workflow picks this runner with runs-on: ["self-hosted","matsu-vm"] (ci.yml).
LABELS='["self-hosted","macOS","ARM64","matsu-vm"]'
# The jobs that the VM lets run (guest/admit.sh). Phase 1 runs only test (macos-arm64) here, but the E2E jobs of
# phase 2 are already admitted: which jobs come here is the workflows' choice; this list only bounds it.
ALLOWED_JOBS='ci.yml:test e2e.yml:smoke e2e.yml:screens'

# Cirrus Labs' vanilla macOS 26 image, pinned by digest. The golden VM starts from it.
VANILLA_IMAGE=ghcr.io/cirruslabs/macos-tahoe-vanilla@sha256:eeec54bfe1f076e27786c5d92b89187a05b1d109b5071eb2dcdf02d596e34640
GOLDEN=matsu-ci-golden
VM_PREFIX=matsu-ci-job-
VM_CPU=4
VM_MEMORY_MB=8192
VM_DISK_GB=100

# The keychain item that holds the fine-grained token (this repository only; Administration and Variables, read and
# write). The user puts it there; the scripts never write it to a file or a log.
KEYCHAIN_SERVICE=matsu-mac-runner

STATE_DIR="$HOME/Library/Application Support/matsu-mac-runner"
LOG="$STATE_DIR/runner.log"

# Limits that the host keeps for each VM (ADR-0027, chapter 7).
JOB_LIMIT_SECONDS=10800      # 3 hours from the start of the job
IDLE_LIMIT_SECONDS=21600     # a VM that got no job in 6 hours is replaced by a fresh one
MIN_FREE_GB=60               # free space needed before a clone
HEARTBEAT_SECONDS=300        # MAC_RUNNER_SEEN; CI falls back to GitHub's runner after 15 minutes without it
# The longest a VM may live, whatever the guest says (it may not answer at all): no job for the idle limit, then a
# job for the job limit.
MAX_VM_SECONDS=$((IDLE_LIMIT_SECONDS + JOB_LIMIT_SECONDS))

SOFTNET=/usr/local/bin/softnet
TART=$(command -v tart 2> /dev/null || true)
[ -n "$TART" ] || TART="$HOME/bin/tart"

die() {
  printf 'mac-runner: %s\n' "$*" >&2
  exit 1
}

log() {
  mkdir -p "$STATE_DIR"
  printf '%s %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$*" >> "$LOG"
}

require_tools() {
  [ -x "$TART" ] || die "tart not found"
  [ -x /usr/bin/jq ] || die "/usr/bin/jq not found (macOS 15 or later has it)"
  # Softnet must run as root: owned by root with the setuid bit (ADR-0027, decision 3).
  [ -u "$SOFTNET" ] && [ "$(stat -f %u "$SOFTNET")" = 0 ] || die "$SOFTNET is not installed as setuid root (see README.md)"
  case ":$PATH:" in
    *:/usr/local/bin:*) ;;
    *) PATH="/usr/local/bin:$PATH" ;;
  esac
  export PATH
}

# True while this Mac uses a VPN, or when that cannot be told (fails closed). Softnet lets the VM reach any globally
# routable address, which can include the services of the network behind a VPN, and with a full tunnel public_ip
# below would find the VPN's address instead of the home network's (ADR-0027, decision 6). A VPN is a connected VPN
# service of the system, or any IPv4 route through a tunnel interface: the default route of a full tunnel, the
# 0/1 and 128/1 routes of WireGuard's wg-quick, the routes of a split tunnel. The Mac's own utun interfaces carry
# no IPv4 routes.
vpn_active() {
  local routes
  scutil --nc list 2> /dev/null | grep -q '(Connected)' && return 0
  routes=$(netstat -rn -f inet 2> /dev/null) || return 0
  [ -n "$routes" ] || return 0
  printf '%s\n' "$routes" | awk '$4 ~ /^(utun|ipsec|ppp|tun|tap|gif|stf)[0-9]/ {found = 1} END {exit !found}' && return 0
  return 1
}

# The public IPv4 address of the home network, from Cloudflare (the VM's DNS too). Empty when it cannot be found.
public_ip() {
  curl -fsS -m 10 https://1.1.1.1/cdn-cgi/trace 2> /dev/null | sed -n 's/^ip=\([0-9][0-9.]*\)$/\1/p' | head -n 1
}

# The Softnet rules of a VM that runs jobs: besides Softnet's own rule (only globally routable addresses), block the
# host in both directions (its SSH, DNS and Bonjour announcements), multicast, and the public address of the home
# network (a router may forward it to the home network). Fails when the public address cannot be found.
job_block_rules() {
  local ip
  vpn_active && return 1
  ip=$(public_ip)
  [ -n "$ip" ] || return 1
  printf 'out @host,in @host,224.0.0.0/4,%s/32' "$ip"
}

vm_running() {
  [ "$("$TART" get "$1" --format json 2> /dev/null | /usr/bin/jq -r '.Running // false')" = true ]
}

vm_exists() {
  "$TART" get "$1" --format json > /dev/null 2>&1
}

# Runs a command with a time limit (seconds). macOS has no timeout(1); perl's alarm ends the command with SIGALRM.
with_limit() { # seconds command...
  local seconds=$1
  shift
  /usr/bin/perl -e 'alarm shift @ARGV; exec @ARGV or exit 127' "$seconds" "$@"
}

# A command in the VM through the guest agent, with a time limit: a job could stop the agent, and tart exec itself
# has no deadline, so the host must not wait for the guest without one.
guest() { # vm seconds command...
  local vm=$1 seconds=$2
  shift 2
  with_limit "$seconds" "$TART" exec "$vm" "$@"
}

# Waits until the guest agent answers, up to $2 seconds (default 180). A VM that is still not running 30 seconds
# after the start (tart run failed) ends the wait early, and so does a VPN.
wait_agent() {
  local limit=${2:-180} start elapsed
  start=$(date +%s)
  while :; do
    elapsed=$(($(date +%s) - start))
    [ "$elapsed" -lt "$limit" ] || return 1
    vpn_active && return 1
    guest "$1" 20 true > /dev/null 2>&1 && return 0
    if [ "$elapsed" -ge 30 ] && ! vm_running "$1"; then
      return 1
    fi
    sleep 3
  done
}

# Waits until the VM has stopped, up to $2 seconds (default 120).
wait_stopped() {
  local limit=${2:-120} start
  start=$(date +%s)
  while [ $(($(date +%s) - start)) -lt "$limit" ]; do
    vm_running "$1" || return 0
    sleep 2
  done
  return 1
}

# Writes stdin to a file in the VM through the guest agent (no network), atomically, within 60 seconds.
put_file() { # vm path
  with_limit 60 "$TART" exec -i "$1" /bin/sh -c "umask 077; cat > '$2.tmp' && mv '$2.tmp' '$2'"
}

# The token from the keychain. Never printed.
token() {
  security find-generic-password -s "$KEYCHAIN_SERVICE" -w 2> /dev/null
}

# A call of GitHub's REST API with the token. The token goes to curl through stdin (-H @-), not the command line.
#   api METHOD PATH [JSON-BODY-FILE]
api() {
  local method=$1 path=$2 body=${3:-} t
  t=$(token) || return 1
  [ -n "$t" ] || return 1
  if [ -n "$body" ]; then
    printf 'Authorization: Bearer %s\nAccept: application/vnd.github+json\nX-GitHub-Api-Version: 2022-11-28\n' "$t" |
      curl -fsS -m 30 -X "$method" -H @- --data-binary @"$body" "https://api.github.com$path"
  else
    printf 'Authorization: Bearer %s\nAccept: application/vnd.github+json\nX-GitHub-Api-Version: 2022-11-28\n' "$t" |
      curl -fsS -m 30 -X "$method" -H @- "https://api.github.com$path"
  fi
}

# Sets a repository variable of Actions, creating it when it does not exist.
set_variable() { # name value
  local f status
  f=$(mktemp "$STATE_DIR/var.XXXXXX")
  /usr/bin/jq -n --arg n "$1" --arg v "$2" '{name: $n, value: $v}' > "$f"
  api PATCH "/repos/$REPOSITORY/actions/variables/$1" "$f" > /dev/null 2>&1 ||
    api POST "/repos/$REPOSITORY/actions/variables" "$f" > /dev/null
  status=$?
  rm -f "$f"
  return $status
}
