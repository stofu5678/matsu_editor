#!/usr/bin/env python3
"""Fill a version and the SHA-256s of a published release into the package-manager files (ADR-0032).

Files it rewrites, in place:
  packaging/homebrew/matsu-editor.rb              (the .dmg)
  packaging/scoop/matsu-editor.json               (the two portable zips)
  packaging/winget/stofu5678.MATSUEditor*.yaml    (the two setup.exe files, and the version in every file)

Usage:
  tools/packaging/update-manifests.py 0.14.0                       # downloads the assets with `gh`
  tools/packaging/update-manifests.py 0.14.0 --assets-dir dist     # uses local copies (a test, or a mirror)

It only reads: `gh release download` fetches the assets of the release into a temporary folder, and nothing is
uploaded, pushed or submitted anywhere. A file that does not look as expected (a line this script fills is
missing, or appears a different number of times) stops the run before anything is written. Python 3.9+, no
packages.
"""

import argparse
import hashlib
import json
import pathlib
import re
import subprocess
import sys
import tempfile

REPO = "stofu5678/matsu_editor"
ROOT = pathlib.Path(__file__).resolve().parents[2]

HOMEBREW = "packaging/homebrew/matsu-editor.rb"
SCOOP = "packaging/scoop/matsu-editor.json"
WINGET = [
    "packaging/winget/stofu5678.MATSUEditor.yaml",
    "packaging/winget/stofu5678.MATSUEditor.installer.yaml",
    "packaging/winget/stofu5678.MATSUEditor.locale.en-US.yaml",
    "packaging/winget/stofu5678.MATSUEditor.locale.ja-JP.yaml",
]
WINGET_INSTALLER = WINGET[1]

# The release's asset names (docs/release.md). {v} is the tag, for example v0.14.0.
ASSET_RE = re.compile(r"matsu-v\d+\.\d+\.\d+(?:-[0-9A-Za-z.]+)?-(windows-(?:x64|arm64)(?:-setup\.exe|\.zip)|macos-arm64\.dmg)$")
VERSION_RE = re.compile(r"^\d+\.\d+\.\d+(?:-[0-9A-Za-z.]+)?$")
SHA_RE = re.compile(r"^[0-9a-f]{64}$")


class Fail(Exception):
    pass


def asset_name(version, suffix):
    return f"matsu-v{version}-{suffix}"


def sha256_of(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def fetch_assets(version, names, assets_dir):
    """Returns {asset name: sha256}. Reads `assets_dir`, or downloads the assets into a temporary folder."""
    if assets_dir:
        hashes = {}
        for name in sorted(names):
            path = pathlib.Path(assets_dir) / name
            if not path.is_file():
                raise Fail(f"{path} is missing")
            hashes[name] = sha256_of(path)
        return hashes
    with tempfile.TemporaryDirectory(prefix="matsu-assets-") as tmp:
        hashes = {}
        for name in sorted(names):
            print(f"downloading {name}", file=sys.stderr)
            cmd = ["gh", "release", "download", f"v{version}", "-R", REPO, "-p", name, "-D", tmp]
            try:
                subprocess.run(cmd, check=True)
            except (OSError, subprocess.CalledProcessError) as e:
                raise Fail(f"could not download {name} of v{version}: {e}")
            hashes[name] = sha256_of(pathlib.Path(tmp) / name)
        return hashes


def sub_once(text, pattern, repl, what, count=1, flags=re.M):
    """Replaces `pattern` and insists that it matched exactly `count` times."""
    new, n = re.subn(pattern, repl, text, flags=flags)
    if n != count:
        raise Fail(f"{what}: expected {count} match(es) of {pattern!r}, found {n}")
    return new


def fill_homebrew(text, version, hashes):
    name = asset_name(version, "macos-arm64.dmg")
    text = sub_once(text, r'^(  version ")[^"]*(")$', rf"\g<1>{version}\g<2>", HOMEBREW)
    text = sub_once(text, r'^(  sha256 ")[0-9a-f]{64}(")$', rf"\g<1>{hashes[name]}\g<2>", HOMEBREW)
    # The url uses #{version} and stays as it is; check that it still names the asset.
    if f"matsu-v#{{version}}-macos-arm64.dmg" not in text:
        raise Fail(f"{HOMEBREW}: the url no longer names matsu-v#{{version}}-macos-arm64.dmg")
    return text


def fill_scoop(text, version, hashes):
    data = json.loads(text)
    data["version"] = version
    for key, suffix in (("64bit", "windows-x64"), ("arm64", "windows-arm64")):
        entry = data["architecture"][key]
        name = asset_name(version, f"{suffix}.zip")
        entry["url"] = f"https://github.com/{REPO}/releases/download/v{version}/{name}"
        entry["hash"] = hashes[name]
    # autoupdate keeps its $version templates; checkver is Scoop's own.
    return json.dumps(data, indent=4, ensure_ascii=False) + "\n"


def fill_winget_installer(text, version, hashes):
    out = []
    pending = None  # the asset whose InstallerSha256 comes next
    urls = 0
    shas = 0
    for line in text.split("\n"):
        m = re.match(r"^(\s*InstallerUrl: )(\S+)$", line)
        if m:
            if pending:
                raise Fail(f"{WINGET_INSTALLER}: an InstallerUrl has no InstallerSha256")
            base = m.group(2).rsplit("/", 1)[1]
            known = ASSET_RE.search(base)
            if not known:
                raise Fail(f"{WINGET_INSTALLER}: the url {m.group(2)} is not a known asset")
            pending = asset_name(version, known.group(1))
            line = f"{m.group(1)}https://github.com/{REPO}/releases/download/v{version}/{pending}"
            urls += 1
        m = re.match(r'^(\s*InstallerSha256: )"?[0-9A-Za-z]+"?$', line)
        if m:
            if not pending:
                raise Fail(f"{WINGET_INSTALLER}: an InstallerSha256 has no InstallerUrl before it")
            # Quoted: a hash of only numbers (or like 1e5...) would be a number to a YAML parser.
            line = f'{m.group(1)}"{hashes[pending]}"'
            pending = None
            shas += 1
        out.append(line)
    if urls != 4 or shas != 4 or pending:
        raise Fail(f"{WINGET_INSTALLER}: expected 4 installers, found {urls} urls and {shas} hashes")
    return "\n".join(out)


def fill_winget_version(text, version, name):
    text = sub_once(text, r"^(PackageVersion: )\S+$", rf"\g<1>{version}", name)
    # The release notes link of the default locale.
    text = re.sub(r"(releases/tag/v)\d+\.\d+\.\d+(?:-[0-9A-Za-z.]+)?$", rf"\g<1>{version}", text, flags=re.M)
    return text


def needed_assets(version):
    return {
        asset_name(version, "macos-arm64.dmg"),
        asset_name(version, "windows-x64.zip"),
        asset_name(version, "windows-arm64.zip"),
        asset_name(version, "windows-x64-setup.exe"),
        asset_name(version, "windows-arm64-setup.exe"),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("version", help="the release version without the v, for example 0.14.0")
    parser.add_argument("--assets-dir", help="read the assets from this folder instead of downloading them")
    parser.add_argument("--root", default=str(ROOT), help="the repository (default: the one of this script)")
    args = parser.parse_args()

    version = args.version.removeprefix("v")
    if not VERSION_RE.match(version):
        parser.error(f"{args.version!r} is not a version like 0.14.0")
    root = pathlib.Path(args.root)

    try:
        hashes = fetch_assets(version, needed_assets(version), args.assets_dir)
        for name, digest in hashes.items():
            if not SHA_RE.match(digest):
                raise Fail(f"bad digest for {name}")
        results = {}  # path -> new text; written only when every file went through
        for rel in [HOMEBREW, SCOOP, *WINGET]:
            text = (root / rel).read_text(encoding="utf-8")
            if rel == HOMEBREW:
                new = fill_homebrew(text, version, hashes)
            elif rel == SCOOP:
                new = fill_scoop(text, version, hashes)
            else:
                if rel == WINGET_INSTALLER:
                    text = fill_winget_installer(text, version, hashes)
                new = fill_winget_version(text, version, rel)
            results[rel] = new
    except Fail as e:
        print(f"error: {e}", file=sys.stderr)
        return 1

    for rel, new in results.items():
        with open(root / rel, "w", encoding="utf-8", newline="\n") as f:
            f.write(new)
        print(f"updated {rel}")
    for name, digest in sorted(hashes.items()):
        print(f"  {digest}  {name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
