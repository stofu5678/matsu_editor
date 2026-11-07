#!/bin/bash
# Tests of pick.sh: which runner the macOS jobs get (ADR-0027). Needs bash and GNU date (Linux); the lint job of
# ci.yml runs it.
#
#   bash .github/actions/mac-runner/pick-test.sh
here=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# The clock of the tests: `date +%s`, the way pick.sh reads the time, says $FAKE_NOW; anything else (`date -d ...`)
# goes to the real date. A pick.sh that reads the clock another way fails most checks here.
real_date=$(command -v date)
mkdir "$work/bin"
cat > "$work/bin/date" <<EOF
#!/bin/bash
if [ "\$#" -eq 1 ] && [ "\$1" = +%s ]; then echo "\$FAKE_NOW"; else exec "$real_date" "\$@"; fi
EOF
chmod +x "$work/bin/date"

now=1800000000 # 2027-01-15T08:00:00Z
iso() { "$real_date" -u -d "@$1" +%Y-%m-%dT%H:%M:%SZ; }

failures=0
count=0
# check <self-hosted|github> <what> [NAME=value...]: runs pick.sh on a push of this repository by a Mac seen a minute
# ago, with the NAME=value changes, and compares the runner it picks.
check() {
  local want=$1 what=$2 kind mac
  shift 2
  count=$((count + 1))
  : > "$work/out"
  # TZ=UTC as on GitHub's runner: a time without a zone must not pass for one by luck of the local zone.
  env -i PATH="$work/bin:$PATH" TZ=UTC FAKE_NOW="$now" GITHUB_OUTPUT="$work/out" \
    MAC_RUNNER=self-hosted MAC_RUNNER_SEEN=$((now - 60)) EVENT=push HEAD_REPO_ID= REPO_ID=1412713500 \
    HEAD_COMMIT_TIME= "$@" bash "$here/pick.sh" > "$work/log" 2>&1
  kind=$(sed -n 's/^mac_kind=//p' "$work/out")
  mac=$(sed -n 's/^mac=//p' "$work/out")
  case "$kind:$mac" in
    'self-hosted:["self-hosted","matsu-vm"]' | 'github:"macos-15"') ;;
    *) kind="broken output ($(tr '\n' ' ' < "$work/out"))" ;;
  esac
  if [ "$kind" != "$want" ]; then
    failures=$((failures + 1))
    echo "FAIL: $what: want $want, got $kind"
    sed 's/^/  /' "$work/log"
  fi
}

check self-hosted 'a push, the Mac seen a minute ago'
check github 'MAC_RUNNER is github' MAC_RUNNER=github
check github 'MAC_RUNNER is empty' MAC_RUNNER=
check github 'the repository ID is not a number' REPO_ID=abc
check github 'the repository ID is empty' REPO_ID=
check github 'a workflow_dispatch' EVENT=workflow_dispatch
check github 'a pull_request_target' EVENT=pull_request_target
check github 'a workflow_dispatch with the ID of this repository as the head' EVENT=workflow_dispatch \
  HEAD_REPO_ID=1412713500
check self-hosted 'a pull request from this repository' EVENT=pull_request HEAD_REPO_ID=1412713500
check github 'a pull request from a fork' EVENT=pull_request HEAD_REPO_ID=99
check github 'a pull request from a deleted fork' EVENT=pull_request HEAD_REPO_ID=

check github 'no heartbeat' MAC_RUNNER_SEEN=
check github 'a heartbeat that is not a number' MAC_RUNNER_SEEN=soon
check github 'a heartbeat of 13 digits' MAC_RUNNER_SEEN=1800000000000
check github 'a heartbeat that wraps around 64 bits to a minute ago' MAC_RUNNER_SEEN=18446744075509551556
check self-hosted 'a heartbeat 30 seconds ahead (clocks differ)' MAC_RUNNER_SEEN=$((now + 30))
check github 'a heartbeat 2 minutes ahead' MAC_RUNNER_SEEN=$((now + 120))
check self-hosted 'a heartbeat of 899 seconds ago' MAC_RUNNER_SEEN=$((now - 899))
check github 'a heartbeat of 901 seconds ago, no commit time' MAC_RUNNER_SEEN=$((now - 901))
check github 'a heartbeat of 901 seconds ago, a pull request' MAC_RUNNER_SEEN=$((now - 901)) EVENT=pull_request \
  HEAD_REPO_ID=1412713500

# A run that waited: the variables are those of the moment it was created (the commit time, give or take seconds).
check self-hosted 'waited 30 minutes; seen 100 seconds before the commit' \
  MAC_RUNNER_SEEN=$((now - 1900)) HEAD_COMMIT_TIME="$(iso $((now - 1800)))"
check self-hosted 'waited 30 minutes; seen 10 seconds after the commit' \
  MAC_RUNNER_SEEN=$((now - 1790)) HEAD_COMMIT_TIME="$(iso $((now - 1800)))"
check self-hosted 'waited 30 minutes; seen 899 seconds before the commit' \
  MAC_RUNNER_SEEN=$((now - 2699)) HEAD_COMMIT_TIME="$(iso $((now - 1800)))"
check github 'waited 30 minutes; seen 901 seconds before the commit' \
  MAC_RUNNER_SEEN=$((now - 2701)) HEAD_COMMIT_TIME="$(iso $((now - 1800)))"
check self-hosted 'waited 2 hours' MAC_RUNNER_SEEN=$((now - 7300)) HEAD_COMMIT_TIME="$(iso $((now - 7200)))"
check github 'waited 2 hours and a second' MAC_RUNNER_SEEN=$((now - 7301)) HEAD_COMMIT_TIME="$(iso $((now - 7201)))"
check self-hosted 'a commit time with the offset +00:00' MAC_RUNNER_SEEN=$((now - 1900)) \
  HEAD_COMMIT_TIME="$("$real_date" -u -d "@$((now - 1800))" +%Y-%m-%dT%H:%M:%S+00:00)"
check self-hosted 'a commit time in Japan time (+09:00)' MAC_RUNNER_SEEN=$((now - 1900)) \
  HEAD_COMMIT_TIME="$(TZ=Asia/Tokyo "$real_date" -d "@$((now - 1800))" +%Y-%m-%dT%H:%M:%S+09:00)"
check github 'a commit time in Japan time, written as UTC (9 hours ahead)' MAC_RUNNER_SEEN=$((now - 1900)) \
  HEAD_COMMIT_TIME="$(TZ=Asia/Tokyo "$real_date" -d "@$((now - 1800))" +%Y-%m-%dT%H:%M:%SZ)"
check github 'a commit time an hour ahead' MAC_RUNNER_SEEN=$((now - 1000)) HEAD_COMMIT_TIME="$(iso $((now + 3600)))"
check github 'a commit time without a zone' MAC_RUNNER_SEEN=$((now - 1900)) \
  HEAD_COMMIT_TIME="$("$real_date" -u -d "@$((now - 1800))" +%Y-%m-%dT%H:%M:%S)"
check github 'a commit time that is a word' MAC_RUNNER_SEEN=$((now - 1900)) HEAD_COMMIT_TIME=yesterday
check github 'a commit time that is not a date' MAC_RUNNER_SEEN=$((now - 1900)) HEAD_COMMIT_TIME=2027-13-45T99:99:99Z
check github 'a commit time with a command in it' MAC_RUNNER_SEEN=$((now - 1900)) \
  HEAD_COMMIT_TIME="\$(touch $work/ran)"
if [ -e "$work/ran" ]; then
  failures=$((failures + 1))
  echo 'FAIL: a command in the commit time ran'
fi
check github 'a fresh commit time does not help a pull request from a fork' MAC_RUNNER_SEEN=$((now - 1900)) \
  HEAD_COMMIT_TIME="$(iso $((now - 1800)))" EVENT=pull_request HEAD_REPO_ID=99
check github 'a fresh commit time does not help MAC_RUNNER=github' MAC_RUNNER=github MAC_RUNNER_SEEN=$((now - 1900)) \
  HEAD_COMMIT_TIME="$(iso $((now - 1800)))"

echo "$count checks, $failures failed"
[ "$failures" -eq 0 ]
