#!/usr/bin/env bash
# An example check for .github/workflows/mac-check.yml: shows what the runner gives (the macOS
# version, SIP, the privacy permissions in the system TCC database) and takes a picture of the
# desktop. Pictures go to $UME_E2E_SCREENS_DIR.
set -euo pipefail
sw_vers
csrutil status || true
sudo sqlite3 -readonly "/Library/Application Support/com.apple.TCC/TCC.db" \
  "select service, client, auth_value from access where service in ('kTCCServiceScreenCapture','kTCCServiceAccessibility','kTCCServicePostEvent')"
screencapture -x "${UME_E2E_SCREENS_DIR:-.}/desktop.png"
