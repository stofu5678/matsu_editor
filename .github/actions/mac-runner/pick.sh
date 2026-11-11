#!/usr/bin/env bash
# Picks the runner of the macOS jobs (ADR-0027) for action.yml next to it: the self-hosted runner in the VM on the
# maintainer's Mac, or GitHub's macos-15. Reads the inputs from the environment (MAC_RUNNER, MAC_RUNNER_SEEN, EVENT,
# HEAD_REPO_ID, REPO_ID, HEAD_COMMIT_TIME, HEAD_COMMIT_COMMITTER) and writes `mac` and `mac_kind` to $GITHUB_OUTPUT.
# Fails closed: anything missing or malformed means GitHub's runner. Runs on GitHub's Ubuntu runner (GNU date).
# Tests: pick-test.sh.

reason=
how='seen recently'
if [ "${MAC_RUNNER}" != self-hosted ]; then
  reason="MAC_RUNNER is '${MAC_RUNNER}', not 'self-hosted'"
elif ! [[ "${REPO_ID}" =~ ^[0-9]{1,20}$ ]]; then
  reason="the repository ID is missing or malformed ('${REPO_ID}')"
elif [ "${EVENT}" = push ]; then
  :
elif [ "${EVENT}" != pull_request ]; then
  reason="the event is '${EVENT}', not push or pull_request"
elif [ "${HEAD_REPO_ID}" != "${REPO_ID}" ]; then
  reason="the pull request comes from another repository ('${HEAD_REPO_ID}')"
fi

if [ -z "${reason}" ]; then
  now=$(date +%s)
  if ! [[ "${MAC_RUNNER_SEEN}" =~ ^[0-9]{1,12}$ ]]; then
    reason="MAC_RUNNER_SEEN is missing or not a number ('${MAC_RUNNER_SEEN}')"
  else
    seen=$((10#${MAC_RUNNER_SEEN}))
    if [ "${seen}" -gt $((now + 60)) ]; then
      reason="MAC_RUNNER_SEEN is in the future ($((seen - now)) seconds ahead)"
    elif [ "${seen}" -lt $((now - 900)) ]; then
      # A run reads the variables as they were when it was created, so a run that waited (a run of release.yml waits
      # for the one before it, in the `release` group) sees the heartbeat of that moment. Then the heartbeat counts
      # against the time of the pushed commit, a few seconds before the run was created, when GitHub made that commit
      # (committer noreply@github.com: a merge on GitHub, whose time is the time of the merge). main takes nothing
      # else (the repository allows only squash merges, and its protection covers the admins too); any other commit
      # may carry an older date, so it does not count.
      # - The heartbeat must be from before the run was created (a minute of slack). A newer one was read later, by a
      #   re-run for example, or the commit is older than the run; then the rule above stands.
      # - The commit must be at most 2 hours old: a longer wait is not normal (every job has a time limit of an hour
      #   at most), and the Mac may be closed by now.
      # `why` says, in the log, why the commit did not help (nothing for a pull request, which has no commit time).
      created=
      why=
      iso='^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(Z|[+-][0-9]{2}:[0-9]{2})$'
      if [ -z "${HEAD_COMMIT_TIME}" ]; then
        :
      elif [ "${HEAD_COMMIT_COMMITTER}" != noreply@github.com ]; then
        why="GitHub did not make the pushed commit (committer '${HEAD_COMMIT_COMMITTER}')"
      elif ! [[ "${HEAD_COMMIT_TIME}" =~ ${iso} ]] || ! created=$(date -d "${HEAD_COMMIT_TIME}" +%s 2> /dev/null) ||
        ! [[ "${created}" =~ ^[0-9]{1,12}$ ]]; then
        created=
        why="the time of the pushed commit is not a date with a zone ('${HEAD_COMMIT_TIME}')"
      elif [ "${seen}" -lt $((created - 900)) ]; then
        why="the heartbeat was $((created - seen)) seconds older than the pushed commit"
      elif [ "${seen}" -gt $((created + 60)) ]; then
        why="the heartbeat is $((seen - created)) seconds newer than the pushed commit (read after the run was created)"
      elif [ "${created}" -gt "${now}" ] || [ $((now - created)) -gt 7200 ]; then
        # (After the two checks above the commit is earlier than now, as the heartbeat is older than 15 minutes; so
        # this is about the 2 hours.)
        why="the pushed commit is not from the last 2 hours ($((now - created)) seconds ago)"
      fi
      if [ -n "${created}" ] && [ -z "${why}" ]; then
        before=$((created > seen ? created - seen : 0))
        waited=$((now - created))
        how="seen ${before} seconds before this run was created, which was ${waited} seconds ago"
      else
        reason="MAC_RUNNER_SEEN is older than 15 minutes ($((now - seen)) seconds)${why:+; ${why}}"
      fi
    fi
  fi
fi

if [ -z "${reason}" ]; then
  echo "macOS runner: self-hosted (MAC_RUNNER=self-hosted, ${how})"
  echo 'mac=["self-hosted","matsu-vm"]' >> "${GITHUB_OUTPUT}"
  echo 'mac_kind=self-hosted' >> "${GITHUB_OUTPUT}"
else
  echo "macOS runner: GitHub's macos-15, because ${reason}"
  echo 'mac="macos-15"' >> "${GITHUB_OUTPUT}"
  echo 'mac_kind=github' >> "${GITHUB_OUTPUT}"
fi
