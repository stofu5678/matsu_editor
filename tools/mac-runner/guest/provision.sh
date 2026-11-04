#!/bin/bash
# Runs inside a new golden VM (as admin, who has passwordless sudo) through `tart exec`, after the guest agent is in.
# Installs only what the CI jobs need, each download checked by SHA-256: the Command Line Tools (the linker and the
# SDK; MATSU compiles its Metal shaders at run time, so no Xcode), rustup with the pinned Rust, and the GitHub Actions
# runner. Then the VM's settings: the public DNS (the host's DNS knows the names of the home network), no Spotlight, no
# SSH server. ADR-0027, chapters 1 and 2.
#
# The files of guest/ must already be in /Users/admin/matsu (build-golden copies them).
set -euo pipefail

RUST_TOOLCHAIN=1.99.0
RUSTUP_VERSION=1.29.2
RUSTUP_SHA256=f7c7e11eb2e07f006e532a8aaac77b32f4527d305ac386c0c633ca6133d35ca9
RUNNER_VERSION=2.338.2
RUNNER_SHA256=25a065af409ffa317ab4d5a3b614dd15fc9c40f2172bc1b35a1b1258e919a388

dir=/Users/admin/matsu
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

step() { printf '\n== %s\n' "$*"; }

step "DNS: Cloudflare's public resolvers instead of the host"
sudo networksetup -setdnsservers Ethernet 1.1.1.1 1.0.0.1
networksetup -getdnsservers Ethernet

step "Spotlight off (the builds write many files)"
sudo mdutil -a -i off > /dev/null || true

step "Command Line Tools"
if ! xcode-select -p > /dev/null 2>&1 || [ ! -x /Library/Developer/CommandLineTools/usr/bin/clang ]; then
  marker=/tmp/.com.apple.dt.CommandLineTools.installondemand.in-progress
  sudo touch "$marker"
  label=$(softwareupdate -l 2>/dev/null | sed -n 's/^\* Label: \(Command Line Tools.*\)$/\1/p' | sort -V | tail -n 1)
  [ -n "$label" ] || { echo "no Command Line Tools in softwareupdate -l"; exit 1; }
  echo "installing: $label"
  sudo softwareupdate -i "$label" --verbose
  sudo rm -f "$marker"
fi
xcode-select -p
clang --version | head -n 1

step "Grow the APFS container to the size of the disk (after softwareupdate)"
/bin/bash "$dir/grow-disk.sh"

step "rustup $RUSTUP_VERSION and Rust $RUST_TOOLCHAIN"
curl -fsSL -o "$work/rustup-init" "https://static.rust-lang.org/rustup/archive/$RUSTUP_VERSION/aarch64-apple-darwin/rustup-init"
echo "$RUSTUP_SHA256  $work/rustup-init" | shasum -a 256 -c -
chmod +x "$work/rustup-init"
"$work/rustup-init" -y --no-modify-path --profile minimal --default-toolchain "$RUST_TOOLCHAIN" --component clippy
# The runner reads PATH from its .env (below); a login shell gets cargo from here.
grep -q 'cargo/env' ~/.zprofile 2>/dev/null || echo '. "$HOME/.cargo/env"' >> ~/.zprofile
~/.cargo/bin/rustc --version
~/.cargo/bin/cargo clippy --version

step "GitHub Actions runner $RUNNER_VERSION"
curl -fsSL -o "$work/runner.tar.gz" "https://github.com/actions/runner/releases/download/v$RUNNER_VERSION/actions-runner-osx-arm64-$RUNNER_VERSION.tar.gz"
echo "$RUNNER_SHA256  $work/runner.tar.gz" | shasum -a 256 -c -
rm -rf ~/actions-runner
mkdir -p ~/actions-runner
tar -xzf "$work/runner.tar.gz" -C ~/actions-runner
# The runner reads .env when it starts: the hook that admits or refuses each job (guest/job-started.sh), and a PATH
# with cargo. Jobs see these too; the hook is the runner's, not the workflow's, so a workflow cannot remove it.
cat > ~/actions-runner/.env <<ENV
ACTIONS_RUNNER_HOOK_JOB_STARTED=$dir/job-started.sh
PATH=/Users/admin/.cargo/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin
LANG=en_US.UTF-8
ENV
~/actions-runner/bin/Runner.Listener --version

step "The runner's launcher and hook"
for f in admit.sh job-started.sh runner-launch.sh settings grow-disk.sh; do
  [ -s "$dir/$f" ] || { echo "missing $dir/$f"; exit 1; }
done
sudo chown root:wheel "$dir/admit.sh" "$dir/job-started.sh" "$dir/runner-launch.sh" "$dir/grow-disk.sh" "$dir/settings"
sudo chmod 755 "$dir/admit.sh" "$dir/job-started.sh" "$dir/runner-launch.sh" "$dir/grow-disk.sh"
sudo chmod 644 "$dir/settings"
mkdir -p ~/Library/LaunchAgents
cp "$dir/io.github.stofu5678.matsu-runner.plist" ~/Library/LaunchAgents/
plutil -lint ~/Library/LaunchAgents/io.github.stofu5678.matsu-runner.plist
rm -f "$dir/jit-config" "$dir/job-started" "$dir/runner.log" "$dir/launch.log"

step "No SSH server (the host talks to the VM through tart exec only)"
sudo launchctl disable system/com.openssh.sshd || true

step "Done"
sw_vers -productVersion
df -h /System/Volumes/Data | tail -n 1
