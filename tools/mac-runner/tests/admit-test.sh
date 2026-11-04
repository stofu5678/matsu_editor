#!/bin/bash
# Tests of guest/admit.sh: which jobs the VM runner lets run (ADR-0027, chapter 3). Needs bash and jq; runs on macOS
# (bash 3.2) and Linux.
#
#   bash tools/mac-runner/tests/admit-test.sh
set -u
here=$(cd "$(dirname "$0")" && pwd)
admit="$here/../guest/admit.sh"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cat > "$work/settings" <<'SETTINGS'
REPOSITORY=stofu5678/matsu_editor
REPOSITORY_ID=1412713500
ALLOW=ci.yml:test
ALLOW=e2e.yml:smoke
ALLOW=e2e.yml:screens
SETTINGS

# Events: the parts of the payload that admit.sh reads.
printf '{"repository":{"id":1412713500}}' > "$work/push.json"
pr() { # file head-repo-id base-repo-id number
  printf '{"repository":{"id":1412713500},"pull_request":{"number":%s,"head":{"repo":%s},"base":{"repo":{"id":%s}}}}' \
    "$4" "$2" "$3" > "$work/$1"
}
pr same.json '{"id":1412713500}' 1412713500 330
pr fork.json '{"id":99}' 1412713500 330
pr deleted-fork.json 'null' 1412713500 330
pr other-base.json '{"id":1412713500}' 77 330
pr bad-number.json '{"id":1412713500}' 1412713500 '"330; rm -rf /"'
printf '{"repository":{"id":99}}' > "$work/other-repo.json"
printf '{"repository":{"id":"1412713500"}}' > "$work/string-id.json"
printf 'not json' > "$work/broken.json"
printf '{"repository":{"id":"1412713500\\n"}}' > "$work/newline-id.json"

failures=0
count=0
# check <expected allow|refuse> <name> <env assignments...>
check() {
  expected=$1
  name=$2
  shift 2
  count=$((count + 1))
  out=$(env -i PATH=/usr/bin:/bin "$@" bash "$admit" "$work/settings" 2>&1)
  status=$?
  case "$expected:$status" in
    allow:0 | refuse:1) ;;
    *)
      failures=$((failures + 1))
      printf 'FAIL %s: expected %s, got exit %s (%s)\n' "$name" "$expected" "$status" "$out"
      return
      ;;
  esac
  case "$expected:$out" in
    allow:allow | refuse:refuse:*) printf 'ok   %s: %s\n' "$name" "$out" ;;
    *)
      failures=$((failures + 1))
      printf 'FAIL %s: unexpected output %s\n' "$name" "$out"
      ;;
  esac
}

base="GITHUB_REPOSITORY=stofu5678/matsu_editor GITHUB_REPOSITORY_ID=1412713500"
push="$base GITHUB_EVENT_NAME=push GITHUB_REF=refs/heads/main GITHUB_EVENT_PATH=$work/push.json"
pull="$base GITHUB_EVENT_NAME=pull_request GITHUB_REF=refs/pull/330/merge"
ci_main="GITHUB_WORKFLOW_REF=stofu5678/matsu_editor/.github/workflows/ci.yml@refs/heads/main"
ci_pr="GITHUB_WORKFLOW_REF=stofu5678/matsu_editor/.github/workflows/ci.yml@refs/pull/330/merge"
e2e_pr="GITHUB_WORKFLOW_REF=stofu5678/matsu_editor/.github/workflows/e2e.yml@refs/pull/330/merge"

# shellcheck disable=SC2086 # the variables hold several assignments on purpose
{
check allow  "push to main, ci.yml test"            $push $ci_main GITHUB_JOB=test
check allow  "same-repo PR, ci.yml test"            $pull GITHUB_EVENT_PATH=$work/same.json $ci_pr GITHUB_JOB=test
check allow  "same-repo PR, e2e.yml smoke"          $pull GITHUB_EVENT_PATH=$work/same.json $e2e_pr GITHUB_JOB=smoke
check allow  "same-repo PR, e2e.yml screens"        $pull GITHUB_EVENT_PATH=$work/same.json $e2e_pr GITHUB_JOB=screens
check refuse "fork PR"                              $pull GITHUB_EVENT_PATH=$work/fork.json $ci_pr GITHUB_JOB=test
check refuse "PR from a deleted fork"               $pull GITHUB_EVENT_PATH=$work/deleted-fork.json $ci_pr GITHUB_JOB=test
check refuse "PR into another repository"          $pull GITHUB_EVENT_PATH=$work/other-base.json $ci_pr GITHUB_JOB=test
check refuse "PR number that is not a number"       $pull GITHUB_EVENT_PATH=$work/bad-number.json $ci_pr GITHUB_JOB=test
check refuse "push to another branch"               $base GITHUB_EVENT_NAME=push GITHUB_REF=refs/heads/feature GITHUB_EVENT_PATH=$work/push.json GITHUB_WORKFLOW_REF=stofu5678/matsu_editor/.github/workflows/ci.yml@refs/heads/feature GITHUB_JOB=test
check refuse "push of a tag"                        $base GITHUB_EVENT_NAME=push GITHUB_REF=refs/tags/v1 GITHUB_EVENT_PATH=$work/push.json GITHUB_WORKFLOW_REF=stofu5678/matsu_editor/.github/workflows/ci.yml@refs/tags/v1 GITHUB_JOB=test
check refuse "pull_request_target"                  $base GITHUB_EVENT_NAME=pull_request_target GITHUB_REF=refs/heads/main GITHUB_EVENT_PATH=$work/same.json $ci_main GITHUB_JOB=test
check refuse "workflow_dispatch"                    $base GITHUB_EVENT_NAME=workflow_dispatch GITHUB_REF=refs/heads/main GITHUB_EVENT_PATH=$work/push.json $ci_main GITHUB_JOB=test
check refuse "release workflow"                     $push GITHUB_WORKFLOW_REF=stofu5678/matsu_editor/.github/workflows/release.yml@refs/heads/main GITHUB_JOB=test
check refuse "package workflow"                     $pull GITHUB_EVENT_PATH=$work/same.json GITHUB_WORKFLOW_REF=stofu5678/matsu_editor/.github/workflows/package.yml@refs/pull/330/merge GITHUB_JOB=package
check refuse "another job of ci.yml"                $push $ci_main GITHUB_JOB=lint
check refuse "a test job of e2e.yml"                $pull GITHUB_EVENT_PATH=$work/same.json $e2e_pr GITHUB_JOB=test
check refuse "job name with a colon"                $push $ci_main "GITHUB_JOB=test:x"
check refuse "workflow of another repository"       $push GITHUB_WORKFLOW_REF=someone/else/.github/workflows/ci.yml@refs/heads/main GITHUB_JOB=test
check refuse "workflow ref of another branch"       $push GITHUB_WORKFLOW_REF=stofu5678/matsu_editor/.github/workflows/ci.yml@refs/heads/feature GITHUB_JOB=test
check refuse "workflow in a subfolder"              $push GITHUB_WORKFLOW_REF=stofu5678/matsu_editor/.github/workflows/x/ci.yml@refs/heads/main GITHUB_JOB=test
check refuse "another repository"                   GITHUB_REPOSITORY=someone/else GITHUB_REPOSITORY_ID=1412713500 GITHUB_EVENT_NAME=push GITHUB_REF=refs/heads/main GITHUB_EVENT_PATH=$work/push.json $ci_main GITHUB_JOB=test
check refuse "another repository ID"                GITHUB_REPOSITORY=stofu5678/matsu_editor GITHUB_REPOSITORY_ID=1 GITHUB_EVENT_NAME=push GITHUB_REF=refs/heads/main GITHUB_EVENT_PATH=$work/push.json $ci_main GITHUB_JOB=test
check refuse "event of another repository"          $base GITHUB_EVENT_NAME=push GITHUB_REF=refs/heads/main GITHUB_EVENT_PATH=$work/other-repo.json $ci_main GITHUB_JOB=test
check refuse "repository ID as a string"            $base GITHUB_EVENT_NAME=push GITHUB_REF=refs/heads/main GITHUB_EVENT_PATH=$work/string-id.json $ci_main GITHUB_JOB=test
check refuse "repository ID with a newline"         $base GITHUB_EVENT_NAME=push GITHUB_REF=refs/heads/main GITHUB_EVENT_PATH=$work/newline-id.json $ci_main GITHUB_JOB=test
check refuse "event file that is not JSON"          $base GITHUB_EVENT_NAME=push GITHUB_REF=refs/heads/main GITHUB_EVENT_PATH=$work/broken.json $ci_main GITHUB_JOB=test
check refuse "no event file"                        $base GITHUB_EVENT_NAME=push GITHUB_REF=refs/heads/main GITHUB_EVENT_PATH=$work/missing.json $ci_main GITHUB_JOB=test
check refuse "nothing set"
}

# The settings themselves.
count=$((count + 1))
if out=$(env -i PATH=/usr/bin:/bin bash "$admit" "$work/missing-settings" 2>&1); then
  failures=$((failures + 1)); echo "FAIL missing settings: $out"
else
  echo "ok   missing settings: $out"
fi

echo "$count checks, $failures failed"
[ "$failures" -eq 0 ]
