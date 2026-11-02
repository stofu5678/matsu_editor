#!/bin/bash
# Started by launchd in the admin's GUI session (io.github.stofu5678.matsu-runner.plist), so that the E2E jobs can
# use the screen. Waits for the host to hand over the just-in-time configuration of the runner through `tart exec`
# (not through the network), runs the runner for that one job, and powers the VM off when the runner ends.
dir=/Users/admin/matsu
config=$dir/jit-config
until [ -s "$config" ]; do
  sleep 1
done
jit=$(/bin/cat "$config")
/bin/rm -f "$config"
cd /Users/admin/actions-runner || exit 1
./run.sh --jitconfig "$jit" >> "$dir/runner.log" 2>&1
echo "runner ended with $?" >> "$dir/runner.log"
/usr/bin/sudo -n /sbin/shutdown -h now
