#!/usr/bin/env bash
# Picks the runner of the macOS jobs (ADR-0027) for action.yml next to it: the self-hosted runner in the VM on the
# maintainer's Mac, or GitHub's macos-15. Reads the inputs from the environment (MAC_RUNNER, MAC_RUNNER_SEEN, EVENT,
# HEAD_REPO_ID, REPO_ID, HEAD_COMMIT_TIME) and writes `mac` and `mac_kind` to $GITHUB_OUTPUT. Fails closed: anything
# missing or malformed means GitHub's runner. Runs on GitHub's Ubuntu runner (GNU date). Tests: pick-test.sh.

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
      # against the time of the pushed commit, a few seconds before the run was created, if that is at most 2 hours
      # ago: a longer wait is not normal (every job has a time limit of an hour at most), and the Mac may be closed
      # by now.
      created=
      if [[ "${HEAD_COMMIT_TIME}" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(Z|[+-][0-9]{2}:[0-9]{2})$ ]]; then
        created=$(date -d "${HEAD_COMMIT_TIME}" +%s 2> /dev/null) || created=
      fi
      # (The commit cannot be later than now here: it is at most 15 minutes after the heartbeat, which is older.)
      if [[ "${created}" =~ ^[0-9]{1,12}$ ]] && [ $((now - created)) -le 7200 ] &&
        [ "${seen}" -ge $((created - 900)) ]; then
        how="seen $((created > seen ? created - seen : 0)) seconds before this run was created, $((now - created)) seconds ago"
      else
        reason="MAC_RUNNER_SEEN is older than 15 minutes ($((now - seen)) seconds)"
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
