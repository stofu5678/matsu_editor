#!/bin/bash
# Decides whether the runner in this VM may run the job (ADR-0027, chapter 3). The job-started hook calls it before
# any step of the job runs.
#
#   admit.sh <settings file>
#
# Prints "allow" and exits 0, or prints "refuse: <reason>" and exits 1. Anything missing, malformed or unexpected is
# a refusal. It reads only the GITHUB_* variables that the runner sets (a workflow cannot override them) and the
# event file, and never lets the shell evaluate their values: they are compared as strings, and the event file is
# read with jq. Runs on macOS's own bash 3.2 and jq.
set -u

refuse() {
  printf 'refuse: %s\n' "$1"
  exit 1
}

settings=${1:-}
[ -n "$settings" ] && [ -f "$settings" ] || refuse "no settings file"
# The settings are fixed in the image: REPOSITORY=owner/name, REPOSITORY_ID=<digits>, and one ALLOW line per job that
# may run here: ALLOW=<workflow file>:<job> lets the job run for a push to main and for a pull request from this
# repository; ALLOW=<workflow file>:<job>:<event> (push or pull_request) lets it run for that event only.
repository=$(sed -n 's/^REPOSITORY=\([A-Za-z0-9_.-]*\/[A-Za-z0-9_.-]*\)$/\1/p' "$settings" | head -n 1)
repository_id=$(sed -n 's/^REPOSITORY_ID=\([0-9][0-9]*\)$/\1/p' "$settings" | head -n 1)
[ -n "$repository" ] || refuse "no REPOSITORY in the settings"
[ -n "$repository_id" ] || refuse "no REPOSITORY_ID in the settings"

jq=/usr/bin/jq
[ -x "$jq" ] || jq=$(command -v jq) || refuse "no jq"

# The repository of the job.
[ "${GITHUB_REPOSITORY:-}" = "$repository" ] || refuse "repository is not $repository"
[ "${GITHUB_REPOSITORY_ID:-}" = "$repository_id" ] || refuse "repository ID is not $repository_id"

event_path=${GITHUB_EVENT_PATH:-}
[ -n "$event_path" ] && [ -f "$event_path" ] || refuse "no event file"
# One number of the event file (the IDs and the pull request number are numbers in GitHub's payloads), or nothing
# when it is missing, is not a number, or the file is not JSON.
event_number() {
  "$jq" -r "($1) | select(type == \"number\") | tostring" "$event_path" 2>/dev/null | head -n 1
}
[ "$(event_number .repository.id)" = "$repository_id" ] || refuse "event of another repository"

# The event: a push to main, or a pull request whose branch is in this repository (not a fork).
case "${GITHUB_EVENT_NAME:-}" in
  push)
    [ "${GITHUB_REF:-}" = refs/heads/main ] || refuse "push to a branch other than main"
    ref_suffix=refs/heads/main
    ;;
  pull_request)
    [ "$(event_number .pull_request.head.repo.id)" = "$repository_id" ] || refuse "pull request from another repository"
    [ "$(event_number .pull_request.base.repo.id)" = "$repository_id" ] || refuse "pull request into another repository"
    number=$(event_number .pull_request.number)
    case "$number" in
      '' | *[!0-9]*) refuse "no pull request number" ;;
    esac
    [ "${GITHUB_REF:-}" = "refs/pull/$number/merge" ] || refuse "unexpected ref for a pull request"
    ref_suffix="refs/pull/$number/merge"
    ;;
  *)
    refuse "event is not push or pull_request"
    ;;
esac

# The workflow and the job: only the pairs that the settings allow.
workflow_ref=${GITHUB_WORKFLOW_REF:-}
prefix="$repository/.github/workflows/"
case "$workflow_ref" in
  "$prefix"*) ;;
  *) refuse "workflow of another repository" ;;
esac
rest=${workflow_ref#"$prefix"}
workflow_file=${rest%%@*}
[ "$rest" = "$workflow_file@$ref_suffix" ] || refuse "workflow ref does not match the event"
case "$workflow_file" in
  '' | */* | *[!A-Za-z0-9_.-]*) refuse "malformed workflow file name" ;;
esac
job=${GITHUB_JOB:-}
case "$job" in
  '' | *[!A-Za-z0-9_-]*) refuse "malformed job name" ;;
esac
event=${GITHUB_EVENT_NAME:-}
grep -Fqx -e "ALLOW=$workflow_file:$job" -e "ALLOW=$workflow_file:$job:$event" "$settings" \
  || refuse "job $workflow_file:$job is not allowed here for the event $event"

printf 'allow\n'
exit 0
