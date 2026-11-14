#!/usr/bin/env python3
"""Checks the package-manager files in packaging/ (ADR-0032), offline except for the winget schemas.

  - packaging/scoop/matsu-editor.json parses as JSON; the winget files parse as YAML
  - the winget files validate against Microsoft's manifest schemas (version 1.28.0). The schemas are fetched from a
    pinned commit of microsoft/winget-cli (MIT) and checked against the SHA-256 below; a folder given with
    --schemas replaces the download
  - the files agree with each other and with the rest of the repository: one version, the ProductCode that
    packaging/windows/matsu.iss gives (its AppId), URLs that name the assets of docs/release.md, no `zap` in the cask
    and no `persist` in the Scoop manifest (both would touch the user's data: ADR-0032, decision 3)

Needs PyYAML and jsonschema (CI installs them into a virtual environment). The Homebrew cask is Ruby; `ruby -c`
checks its syntax in CI.
"""

import argparse
import hashlib
import json
import pathlib
import re
import sys
import urllib.request

import jsonschema
import yaml

ROOT = pathlib.Path(__file__).resolve().parents[2]
WINGET_DIR = "packaging/winget"
SCOOP = "packaging/scoop/matsu-editor.json"
HOMEBREW = "packaging/homebrew/matsu-editor.rb"
ISS = "packaging/windows/matsu.iss"

SCHEMA_COMMIT = "57d424f360608411e871c44c7d0ba21d848bcab6"
SCHEMA_URL = (
    "https://raw.githubusercontent.com/microsoft/winget-cli/"
    + SCHEMA_COMMIT
    + "/schemas/JSON/manifests/v1.28.0/manifest.{kind}.1.28.0.json"
)
# kind (as in the file name) -> (the winget file that uses it, SHA-256 of the schema file)
SCHEMAS = {
    "version": ("stofu5678.MATSUEditor.yaml", "5c9fa37ebe70dd410114157d31e4d9e84451b2bb8890e1c4345b4c690406ee1a"),
    "installer": (
        "stofu5678.MATSUEditor.installer.yaml",
        "2caa4cd3a630c284b5ef750522e2e5861f751f376180b89cd29873eb30504ef4",
    ),
    "defaultLocale": (
        "stofu5678.MATSUEditor.locale.en-US.yaml",
        "734c069c82470e3436d274fb381bfbe03e22f8fbfae2d2dd50df7dc055a3e061",
    ),
    "locale": (
        "stofu5678.MATSUEditor.locale.ja-JP.yaml",
        "7d55396c4b88ce2165692d71685eab93189ba24cb20368b3903965d6f91c267a",
    ),
}

problems = []


def check(ok, message):
    if not ok:
        problems.append(message)


def load_schema(kind, schemas_dir):
    _, expected = SCHEMAS[kind]
    if schemas_dir:
        data = (pathlib.Path(schemas_dir) / f"manifest.{kind}.1.28.0.json").read_bytes()
    else:
        with urllib.request.urlopen(SCHEMA_URL.format(kind=kind), timeout=60) as response:
            data = response.read()
    got = hashlib.sha256(data).hexdigest()
    if got != expected:
        raise SystemExit(f"the {kind} schema has SHA-256 {got}, expected {expected}")
    return json.loads(data)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--schemas", help="a folder with manifest.<kind>.1.28.0.json instead of downloading them")
    parser.add_argument("--root", default=str(ROOT))
    args = parser.parse_args()
    root = pathlib.Path(args.root)

    # Parse.
    scoop_text = (root / SCOOP).read_text(encoding="utf-8")
    scoop = json.loads(scoop_text)
    winget = {}
    for kind, (name, _) in SCHEMAS.items():
        winget[kind] = yaml.safe_load((root / WINGET_DIR / name).read_text(encoding="utf-8"))

    # The winget manifests against the schemas.
    for kind in SCHEMAS:
        schema = load_schema(kind, args.schemas)
        validator = jsonschema.Draft7Validator(schema)
        for error in validator.iter_errors(winget[kind]):
            where = "/".join(str(p) for p in error.absolute_path)
            check(False, f"winget {SCHEMAS[kind][0]}: {where}: {error.message}")

    # Agreement.
    versions = {name: m.get("PackageVersion") for name, m in ((k, winget[k]) for k in winget) if "PackageVersion" in m}
    versions["scoop"] = scoop.get("version")
    cask = (root / HOMEBREW).read_text(encoding="utf-8")
    m = re.search(r'^  version "([^"]+)"$', cask, re.M)
    versions["homebrew"] = m.group(1) if m else None
    check(len(set(versions.values())) == 1, f"the files name different versions: {versions}")
    version = scoop.get("version")

    iss = (root / ISS).read_text(encoding="utf-8")
    app_id = re.search(r"^AppId=\{\{([0-9A-Fa-f-]{36})\}$", iss, re.M)
    check(app_id is not None, f"{ISS}: no AppId line")
    if app_id:
        check(
            winget["installer"].get("ProductCode") == "{" + app_id.group(1) + "}_is1",
            f"winget ProductCode is not the AppId of {ISS} plus _is1",
        )
    check(winget["installer"].get("InstallerType") == "inno", "winget InstallerType is not inno")
    pairs = set()
    for installer in winget["installer"]["Installers"]:
        arch = installer["Architecture"]
        pairs.add((arch, installer.get("Scope")))
        want = f"matsu-v{version}-windows-{arch}-setup.exe"
        check(installer["InstallerUrl"].endswith(f"/download/v{version}/{want}"), f"winget URL is not {want}")
    check(
        pairs == {(a, s) for a in ("x64", "arm64") for s in ("user", "machine")},
        f"winget installers are not x64 and arm64 in user and machine scope: {sorted(pairs)}",
    )
    for key, target in (("64bit", "windows-x64"), ("arm64", "windows-arm64")):
        entry = scoop["architecture"][key]
        want = f"matsu-v{version}-{target}.zip"
        check(entry["url"].endswith(f"/download/v{version}/{want}"), f"scoop {key} URL is not {want}")
        check(entry.get("extract_dir") == f"matsu-{target}", f"scoop {key} extract_dir is not matsu-{target}")
        check(re.fullmatch(r"[0-9a-f]{64}", entry["hash"]) is not None, f"scoop {key} hash is not a SHA-256")
    check("persist" not in scoop, "scoop has `persist`: it would put user data in the install folder")
    check(scoop.get("bin") == "matsu.exe", "scoop bin is not matsu.exe")
    check(
        f"matsu-v#{{version}}-macos-arm64.dmg" in cask and "arm64" in cask,
        "the cask does not name matsu-v#{version}-macos-arm64.dmg",
    )
    check(re.search(r"^\s*zap\b", cask, re.M) is None, "the cask has `zap`: it would delete the user's settings")
    check(re.search(r"^  sha256 \"[0-9a-f]{64}\"$", cask, re.M) is not None, "the cask has no sha256")
    check('app "MATSU Editor.app"' in cask, 'the cask has no app "MATSU Editor.app"')

    if problems:
        for p in problems:
            print(f"error: {p}", file=sys.stderr)
        return 1
    print(f"ok: scoop, winget and homebrew files are consistent (version {version})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
