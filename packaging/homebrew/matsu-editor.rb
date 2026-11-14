# Homebrew cask of MATSU Editor for the owner's own tap (ADR-0032, decision 5): copy this file to
# Casks/matsu-editor.rb of a tap. tools/packaging/update-manifests.py sets version and sha256.
cask "matsu-editor" do
  version "0.0.0"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"

  url "https://github.com/stofu5678/matsu_editor/releases/download/v#{version}/matsu-v#{version}-macos-arm64.dmg"
  name "MATSU Editor"
  desc "Text editor that brings the feel of SAKURA Editor to macOS, Windows and Linux"
  homepage "https://github.com/stofu5678/matsu_editor"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on arch: :arm64
  depends_on macos: ">= :big_sur"

  app "MATSU Editor.app"

  # `matsu` starts the `matsu-script` that sits next to its own executable (std::env::current_exe), so a plain
  # symlink into the bundle is not safe. A shim that execs the real path in the bundle is.
  shimscript = "#{staged_path}/matsu"
  binary shimscript, target: "matsu"

  preflight do
    File.write shimscript, <<~EOS
      #!/bin/sh
      exec '#{appdir}/MATSU Editor.app/Contents/MacOS/matsu' "$@"
    EOS
    FileUtils.chmod 0755, shimscript
  end

  # No `zap`: it would delete the user's settings (~/Library/Application Support/matsu-editor), which
  # uninstalling must keep (ADR-0032, decision 3).

  caveats <<~EOS
    MATSU Editor is not signed with an Apple Developer ID yet (it only has an ad-hoc signature), so macOS may say
    it cannot verify the developer. Open it once with right click > Open, or clear the quarantine attribute:

      xattr -dr com.apple.quarantine "#{appdir}/MATSU Editor.app"

    Uninstalling keeps your settings and unsaved documents in ~/Library/Application Support/matsu-editor.
  EOS
end
