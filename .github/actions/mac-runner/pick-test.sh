#!/bin/bash
# Tests of pick.sh: which runner the macOS jobs get (ADR-0027). Needs bash and GNU date (Linux); the lint job of
# ci.yml runs it.
#
#   bash .github/actions/mac-runner/pick-test.sh
here=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# The clock of the tests: `date +%s`, the way pick.sh reads the time, says $FAKE_NOW (and leaves a mark that the end
# checks); anything else (`date -d ...`) goes to the real date. A pick.sh that reads the clock another way fails here.
real_date=$(command -v date)
mkdir "$work/bin"
cat > "$work/bin/date" <<EOF
#!/bin/bash
if [ "\$#" -eq 1 ] && [ "\$1" = +%s ]; then
  touch "$work/clock-read"
  echo "\$FAKE_NOW"
else
  exec "$real_date" "\$@"
fi
EOF
chmod +x "$work/bin/date"

now=1800000000 # 2027-01-15T08:00:00Z
iso() { "$real_date" -u -d "@$1" +%Y-%m-%dT%H:%M:%SZ; }

failures=0
count=0
# check <self-hosted|github> <what> [NAME=value...]: runs pick.sh on a push of this repository (a commit that GitHub
# made) by a Mac seen a minute ago, with the NAME=value changes, and compares the runner it picks.
check() {
  local want=$1 what=$2 kind mac
  shift 2
  count=$((count + 1))
  : > "$work/out"
  # TZ=UTC as on GitHub's runner: a time without a zone must not pass for one by luck of the local zone.
  env -i PATH="$work/bin:$PATH" TZ=UTC FAKE_NOW="$now" GITHUB_OUTPUT="$work/out" \
    MAC_RUNNER=self-hosted MAC_RUNNER_SEEN=$((now - 60)) EVENT=push HEAD_REPO_ID= REPO_ID=1412713500 \
    HEAD_COMMIT_TIME= HEAD_COMMIT_COMMITTER=noreply@github.com "$@" bash "$here/pick.sh" > "$work/log" 2>&1
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

# log_has <regex>: the log of the check before says this (why it picked that runner).
log_has() {
  count=$((count + 1))
  if ! grep -Eq -- "$1" "$work/log"; then
    failures=$((failures + 1))
    echo "FAIL: the log of the check before does not match /$1/"
    sed 's/^/  /' "$work/log"
  fi
}

check self-hosted 'a push, the Mac seen a minute ago'
if [ ! -e "$work/clock-read" ]; then
  echo 'FAIL: pick.sh did not read the clock of the tests (date +%s); the checks would use the real time'
  exit 1
fi
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
log_has 'older than 15 minutes \(901 seconds\)$'

# A run that waited: the variables are those of the moment it was created (the commit time, give or take seconds).
check self-hosted 'waited 30 minutes; seen 100 seconds before the commit' \
  MAC_RUNNER_SEEN=$((now - 1900)) HEAD_COMMIT_TIME="$(iso $((now - 1800)))"
log_has 'seen 100 seconds before this run was created, which was 1800 seconds ago'
check self-hosted 'waited 30 minutes; seen 10 seconds after the commit' \
  MAC_RUNNER_SEEN=$((now - 1790)) HEAD_COMMIT_TIME="$(iso $((now - 1800)))"
check self-hosted 'waited 30 minutes; seen 899 seconds before the commit' \
  MAC_RUNNER_SEEN=$((now - 2699)) HEAD_COMMIT_TIME="$(iso $((now - 1800)))"
check github 'waited 30 minutes; seen 901 seconds before the commit' \
  MAC_RUNNER_SEEN=$((now - 2701)) HEAD_COMMIT_TIME="$(iso $((now - 1800)))"
log_has 'heartbeat was 901 seconds older than the pushed commit'
check self-hosted 'waited 2 hours' MAC_RUNNER_SEEN=$((now - 7300)) HEAD_COMMIT_TIME="$(iso $((now - 7200)))"
check github 'waited 2 hours and a second' MAC_RUNNER_SEEN=$((now - 7301)) HEAD_COMMIT_TIME="$(iso $((now - 7201)))"
log_has 'not from the last 2 hours \(7201 seconds ago\)'
check self-hosted 'waited 30 minutes; seen 60 seconds after the commit' \
  MAC_RUNNER_SEEN=$((now - 1740)) HEAD_COMMIT_TIME="$(iso $((now - 1800)))"
check github 'seen 61 seconds after the commit: read after the run was created' \
  MAC_RUNNER_SEEN=$((now - 1739)) HEAD_COMMIT_TIME="$(iso $((now - 1800)))"
log_has 'heartbeat is 61 seconds newer than the pushed commit'
check github 'a re-run an hour after the merge; the Mac seen 30 minutes after it and closed since' \
  MAC_RUNNER_SEEN=$((now - 1800)) HEAD_COMMIT_TIME="$(iso $((now - 3600)))"
check github 'a commit 40 minutes older than its push; the Mac closed 30 minutes ago' \
  MAC_RUNNER_SEEN=$((now - 1800)) HEAD_COMMIT_TIME="$(iso $((now - 2400)))"
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
log_has "not a date with a zone \\('yesterday'\\)"
check github 'a commit time that is not a date' MAC_RUNNER_SEEN=$((now - 1900)) HEAD_COMMIT_TIME=2027-13-45T99:99:99Z
check github 'a commit time with a command in it' MAC_RUNNER_SEEN=$((now - 1900)) \
  HEAD_COMMIT_TIME="\$(touch $work/ran)"
if [ -e "$work/ran" ]; then
  failures=$((failures + 1))
  echo 'FAIL: a command in the commit time ran'
fi
check github 'waited 30 minutes, but a commit that GitHub did not make' MAC_RUNNER_SEEN=$((now - 1900)) \
  HEAD_COMMIT_TIME="$(iso $((now - 1800)))" HEAD_COMMIT_COMMITTER=someone@example.com
log_has "GitHub did not make the pushed commit \\(committer 'someone@example.com'\\)"
check github 'waited 30 minutes, but no committer' MAC_RUNNER_SEEN=$((now - 1900)) \
  HEAD_COMMIT_TIME="$(iso $((now - 1800)))" HEAD_COMMIT_COMMITTER=
check github 'a fresh commit time does not help a pull request from a fork' MAC_RUNNER_SEEN=$((now - 1900)) \
  HEAD_COMMIT_TIME="$(iso $((now - 1800)))" EVENT=pull_request HEAD_REPO_ID=99
check github 'a fresh commit time does not help MAC_RUNNER=github' MAC_RUNNER=github MAC_RUNNER_SEEN=$((now - 1900)) \
  HEAD_COMMIT_TIME="$(iso $((now - 1800)))"

echo "$count checks, $failures failed"
[ "$failures" -eq 0 ]
