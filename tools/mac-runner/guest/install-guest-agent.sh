#!/bin/bash
# Runs inside the VM (as admin, passwordless sudo). Installs the Tart guest agent from its GitHub release, checked by
# SHA-256, with the same two launchd jobs as Cirrus Labs' base image (data/tart-guest-*.plist), but from /usr/local/bin.
set -euo pipefail
ver=v0.15.0
sum=eb47f402f18e742a8ea96344115ca776179f9c4c67494bc0c46a10a68db280b7
work=$(mktemp -d)
cd "$work"
curl -fsSL -o tga.tar.gz "https://github.com/cirruslabs/tart-guest-agent/releases/download/${ver}/tart-guest-agent-darwin-all.tar.gz"
echo "${sum}  tga.tar.gz" | shasum -a 256 -c -
tar -xzf tga.tar.gz
ls -la
bin=$(find . -type f -name 'tart-guest-agent' | head -1)
sudo mkdir -p /usr/local/bin
sudo install -o root -g wheel -m 755 "$bin" /usr/local/bin/tart-guest-agent
plist() { # label mode workdir
  cat <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
  <dict>
    <key>Label</key><string>$1</string>
    <key>ProgramArguments</key><array><string>/usr/local/bin/tart-guest-agent</string><string>$2</string></array>
    <key>EnvironmentVariables</key><dict><key>PATH</key><string>/bin:/usr/bin:/usr/sbin:/sbin:/usr/local/bin</string></dict>
    <key>WorkingDirectory</key><string>$3</string>
    <key>RunAtLoad</key><true/>
    <key>KeepAlive</key><true/>
    <key>StandardOutPath</key><string>/tmp/$1.log</string>
    <key>StandardErrorPath</key><string>/tmp/$1.log</string>
  </dict>
</plist>
PLIST
}
plist org.cirruslabs.tart-guest-daemon --run-daemon /var/empty | sudo tee /Library/LaunchDaemons/org.cirruslabs.tart-guest-daemon.plist > /dev/null
plist org.cirruslabs.tart-guest-agent --run-agent /Users/admin | sudo tee /Library/LaunchAgents/org.cirruslabs.tart-guest-agent.plist > /dev/null
sudo chown root:wheel /Library/LaunchDaemons/org.cirruslabs.tart-guest-daemon.plist /Library/LaunchAgents/org.cirruslabs.tart-guest-agent.plist
sudo chmod 0644 /Library/LaunchDaemons/org.cirruslabs.tart-guest-daemon.plist /Library/LaunchAgents/org.cirruslabs.tart-guest-agent.plist
sudo launchctl bootstrap system /Library/LaunchDaemons/org.cirruslabs.tart-guest-daemon.plist || true
sudo launchctl bootstrap gui/501 /Library/LaunchAgents/org.cirruslabs.tart-guest-agent.plist || true
sleep 2
/usr/local/bin/tart-guest-agent --version 2>&1 | head -1 || true
launchctl print system/org.cirruslabs.tart-guest-daemon 2>/dev/null | grep -E 'state|pid' | head -2 || true
rm -rf "$work"
