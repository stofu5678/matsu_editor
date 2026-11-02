#!/bin/bash
# The job-started hook of the runner in the VM (ACTIONS_RUNNER_HOOK_JOB_STARTED in the runner's .env). It runs before
# any step of the job. When admit.sh refuses the job, it powers the VM off at once: a failing hook only fails the
# job, and the steps with `if: always()` would still run (ADR-0027, chapter 3). The host sees the VM stop and
# deletes it, with the runner's registration.
#
# The runner runs this script with `bash -e -o pipefail`. With -e, a refusal (admit.sh exits 1) would end the script
# right there, before the power-off, and the job would go on to its `if: always()` steps (seen on 2026-11-02). So
# the first line turns -e off, and the refusal path below never exits before the VM is off.
set +e +o pipefail
dir=/Users/admin/matsu
if verdict=$(/bin/bash "$dir/admit.sh" "$dir/settings" 2>&1) && [ "$verdict" = allow ]; then
  echo "matsu-vm: this job may run on this runner"
  # The host starts the time limit of the job when it sees this file.
  /usr/bin/touch "$dir/job-started"
  exit 0
fi
echo "matsu-vm: this runner refuses the job (${verdict:-no answer}); powering the VM off" >&2
/usr/bin/sudo -n /sbin/halt -q
/usr/bin/sudo -n /sbin/shutdown -h now
# When neither could power off, end the runner itself so that no later step runs.
/usr/bin/sudo -n /usr/bin/pkill -9 -f 'Runner\.(Listener|Worker)'
/usr/bin/pkill -9 -f 'Runner\.(Listener|Worker)'
sleep 600
exit 1
