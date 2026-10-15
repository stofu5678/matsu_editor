#!/usr/bin/env bash
# Builds the harness for x86 and x64 on Windows (from WSL), runs the cases against Sakura's
# bregonig.dll 4.20 and writes bregonig-4.20-x86.json and bregonig-4.20-x64.json next to this
# script. See README.md.
#
# Usage: [BRON_ZIP=path/to/bron420.zip] tools/bregonig-compat/run.sh [bron420.zip] [cases.json]
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
zip="${1:-${BRON_ZIP:-$HOME/project/ume_editor/.claude/upstream/sakura/installer/externals/bregonig/bron420.zip}}"
cases="${2:-$here/cases.json}"

if [ ! -f "$zip" ]; then
    echo "bron420.zip not found: $zip" >&2
    exit 1
fi

# A Windows folder: cmd.exe refuses a UNC current directory.
local_app="$(cd /mnt/c && cmd.exe /c 'echo %LOCALAPPDATA%' | tr -d '\r')"
work_win="$local_app\\Temp\\ume-bregonig-compat"
work="$(wslpath -u "$work_win")"
fw32=/mnt/c/Windows/Microsoft.NET/Framework/v4.0.30319/csc.exe
fw64=/mnt/c/Windows/Microsoft.NET/Framework64/v4.0.30319/csc.exe

rm -rf "$work"
mkdir -p "$work/x86" "$work/x64"
cp "$here/Harness.cs" "$work/Harness.cs"
cp "$cases" "$work/cases.json"

# Only the two DLL files are taken out of the package. The source archive inside is not touched.
unzip -j -o "$zip" bregonig.dll -d "$work/x86" >/dev/null
unzip -j -o "$zip" x64/bregonig.dll -d "$work/x64" >/dev/null

cd "$work"
"$fw32" /nologo /platform:x86 /out:"x86\\Harness.exe" /r:System.Web.Extensions.dll Harness.cs
"$fw64" /nologo /platform:x64 /out:"x64\\Harness.exe" /r:System.Web.Extensions.dll Harness.cs

status=0
for arch in x86 x64; do
    echo "running $arch ..."
    if "./$arch/Harness.exe" "$work_win\\cases.json" "$work_win\\result-$arch.json"; then
        cp "result-$arch.json" "$here/bregonig-4.20-$arch.json"
    else
        echo "$arch failed to run" >&2
        status=1
    fi
done

cd /mnt/c
rm -rf "$work"
exit $status
