#!/bin/bash
# Runs inside the golden VM (as admin, with passwordless sudo). Grows the system's APFS container to the whole disk
# after `tart set --disk-size`.
#
# The first time (before relocate-recovery.py has run on the host), the recovery container sits right after the
# system's container, so only the repair below does something: it moves the backup GPT to the new end of the disk,
# which relocate-recovery.py needs. The second time, the container grows (macOS may already have grown it at boot).
set -uo pipefail
yes | sudo diskutil repairDisk disk0 > /dev/null 2>&1 || true
container=$(diskutil list disk0 | awk '/ Apple_APFS / {print $NF; exit}')
[ -n "$container" ] || { echo "no APFS container on disk0"; exit 1; }
listing=$(diskutil list disk0)
# The line right after the system's container: the recovery partition while it is in the way, or the free space
# once relocate-recovery.py has moved it to the end.
after=$(printf '%s\n' "$listing" | awk -v c="$container" 'found {print; exit} $NF == c {found = 1}')
if ! printf '%s\n' "$listing" | grep -q '(free space)'; then
  echo "no free space on disk0: $container already fills the disk"
elif printf '%s' "$after" | grep -q Apple_APFS_Recovery; then
  echo "the recovery partition is right after $container; relocate-recovery.py moves it after this boot"
else
  sudo diskutil apfs resizeContainer "$container" 0 2>&1 | tail -n 1 || true
fi
diskutil list disk0 | sed -n '3,8p'
df -h /System/Volumes/Data | tail -n 1
