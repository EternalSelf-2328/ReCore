#!/usr/bin/env python3
"""Package ReCore for macOS as a `.app` bundle, a ZIP and, on a Mac, a DMG.

The app carries the same themes as the Windows and Linux packages: a fresh
install shows ReCore, ReCore Paper, Ambxst and the designed CSS themes without
opening Appearance's URL field. The bundle is ad-hoc signed by default; a
distribution build can replace the signature with a Developer ID.

Run it after `scripts/build-macos`; it is also called by that script.
"""
import argparse
import json
import os
import plistlib
import platform
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from release_package import ROOT, dependency_notices, digest, names_home, source_snapshot, zip_directory

FLAVOURS = {
    "discord": {"features": ["--no-default-features", "--features", "desktop,discord"], "suffix": ""},
    "full": {"features": ["--features", "matrix"], "suffix": "-full-preview"},
}
TARGETS = {
    "aarch64-apple-darwin": ("arm64", "Apple Silicon"),
    "x86_64-apple-darwin": ("x64", "Intel"),
}


def target_arch(target):
    if target in TARGETS:
        return TARGETS[target]
    machine = platform.machine().lower()
    return ("arm64" if machine in ("arm64", "aarch64") else "x64", "Mac")


def find_webrtc(target):
    """The libwebrtc headers and archive the build used, for its licence."""
    configured = os.environ.get("LK_CUSTOM_WEBRTC")
    if configured and (Path(configured) / "LICENSE.md").is_file():
        return Path(configured)
    roots = [
        ROOT / "target" / target / "release/build",
        ROOT / "target" / "release/build",
    ]
    for root in roots:
        for scratch in root.glob("scratch-*/out/livekit_webrtc"):
            for licence in scratch.rglob("LICENSE.md"):
                return licence.parent
    raise SystemExit(
        "Could not find libwebrtc's LICENSE.md; set LK_CUSTOM_WEBRTC to its "
        "mac-{arch}-release folder before packaging"
    )


def make_icon(resources):
    """A .icns when the tooling is there; the bundle still opens without it."""
    svg = ROOT / "packaging/icon.svg"
    if not shutil.which("rsvg-convert") or not shutil.which("iconutil"):
        return
    with tempfile.TemporaryDirectory(prefix="recore-iconset-") as temporary:
        iconset = Path(temporary) / "ReCore.iconset"
        iconset.mkdir()
        # iconutil's exact names: each size and its @2x companion, 1024 px
        # for the largest @2x. A mismatched size makes iconutil refuse the set.
        for side in (16, 32, 128, 256, 512):
            for scale, suffix in ((1, ""), (2, "@2x")):
                pixels = side * scale
                png = Path(temporary) / f"icon-{pixels}.png"
                subprocess.run(
                    ["rsvg-convert", "-w", str(pixels), "-h", str(pixels), str(svg), "-o", str(png)],
                    check=True,
                    timeout=30,
                )
                shutil.copy2(png, iconset / f"icon_{side}x{side}{suffix}.png")
        subprocess.run(
            ["iconutil", "-c", "icns", str(iconset), "-o", str(resources / "ReCore.icns")],
            check=True,
            timeout=30,
        )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    native = (
        "aarch64-apple-darwin"
        if platform.machine().lower() in ("arm64", "aarch64")
        else "x86_64-apple-darwin"
    )
    parser.add_argument(
        "--target",
        default=os.environ.get("RECORE_TARGET", native),
        choices=sorted(TARGETS),
    )
    parser.add_argument("--flavour", choices=FLAVOURS, default="discord")
    parser.add_argument("--binary", type=Path, help="default: target/TARGET/release/recore")
    parser.add_argument("--output", type=Path, help="default: target/dist/macos")
    parser.add_argument("--no-dmg", action="store_true", help="make only the app ZIP")
    args = parser.parse_args()
    flavour = FLAVOURS[args.flavour]
    arch, label = target_arch(args.target)
    output = args.output or ROOT / "target/dist/macos"
    output.mkdir(parents=True, exist_ok=True)
    binary = (args.binary or ROOT / "target" / args.target / "release/recore").resolve()
    if not binary.is_file():
        raise SystemExit(f"No macOS binary at {binary}; run scripts/build-macos first")
    contents = binary.read_bytes()
    if names_home(contents):
        raise SystemExit(f"{binary} names {Path.home()}; build it with scripts/build-macos")
    if platform.system() == "Darwin":
        architectures = subprocess.check_output(["lipo", "-archs", str(binary)], text=True, timeout=30).strip()
        expected = {"arm64": "arm64", "x64": "x86_64"}[arch]
        if expected not in architectures.split():
            raise SystemExit(f"{binary} is {architectures}, not {expected}")
    metadata = json.loads(
        subprocess.check_output(
            [
                "cargo", "metadata", "--locked", "--format-version", "1",
                "--filter-platform", args.target, *flavour["features"],
            ],
            cwd=ROOT,
            text=True,
            timeout=30,
        )
    )
    version = next(package["version"] for package in metadata["packages"] if package["name"] == "recore")
    base = f"ReCore-{version}{flavour['suffix']}-macos-{arch}"
    archive_path = output / f"{base}.zip"
    # The source snapshot is the same tracked tree in every job, so it keeps
    # the shared release name rather than one per architecture.
    source_base = f"ReCore-{version}{flavour['suffix']}"
    source_path = output / f"{source_base}-source.zip"
    with tempfile.TemporaryDirectory(prefix="recore-macos-package-") as temporary:
        temporary = Path(temporary)
        rtc = source_snapshot(metadata, source_base, temporary, source_path)
        app = temporary / "ReCore.app"
        macos = app / "Contents/MacOS"
        resources = app / "Contents/Resources"
        macos.mkdir(parents=True)
        resources.mkdir(parents=True)
        shutil.copy2(binary, macos / "recore")
        (macos / "recore").chmod(0o755)
        # The client looks in Contents/Resources/themes for a bundle.
        shutil.copytree(ROOT / "themes", resources / "themes")
        shutil.copy2(ROOT / "LICENSE", resources / "LICENSE")
        (resources / "SOURCE.txt").write_text(
            f"The source code of this build of ReCore {version}"
            + (", with the exact source of\nthe libraries it includes under the AGPL," if rtc else "")
            + " is published with it:\n\n"
            f"  https://github.com/leriart/ReCore/releases/tag/v{version}{flavour['suffix']}\n"
            f"  {source_path.name}, SHA-256 in BUILD.json\n\n"
            "and in the repository, https://github.com/leriart/ReCore. The licenses folder\n"
            "holds every dependency's own license and notices.\n",
            encoding="utf-8",
        )
        dependencies = dependency_notices(metadata, resources, resources / "licenses", find_webrtc(args.target))
        (resources / "THIRD-PARTY.json").write_text(json.dumps(dependencies, indent=2) + "\n", encoding="utf-8")
        info = {
            "CFBundleName": "ReCore",
            "CFBundleDisplayName": "ReCore",
            "CFBundleIdentifier": "io.github.leriart.ReCore",
            "CFBundleExecutable": "recore",
            "CFBundlePackageType": "APPL",
            "CFBundleShortVersionString": version,
            "CFBundleVersion": version.split("-")[0],
            "LSMinimumSystemVersion": "11.0",
            "NSHighResolutionCapable": True,
            "NSPrincipalClass": "NSApplication",
            "CFBundleDocumentTypes": [],
        }
        if (resources / "ReCore.icns").is_file():
            info["CFBundleIconFile"] = "ReCore.icns"
        (app / "Contents/Info.plist").write_bytes(plistlib.dumps(info))
        try:
            make_icon(resources)
        except (OSError, subprocess.CalledProcessError) as error:
            # An icon is nice, not a reason to withhold the release.
            print(f"macOS icon skipped: {error}", file=sys.stderr)
        if (resources / "ReCore.icns").is_file():
            info["CFBundleIconFile"] = "ReCore.icns"
            (app / "Contents/Info.plist").write_bytes(plistlib.dumps(info))
        (app / "Contents/PkgInfo").write_text("APPL????", encoding="ascii")
        (resources / "README.txt").write_text(
            f"ReCore {version} for macOS ({label})\n\n"
            "1. Drag ReCore.app to Applications.\n"
            "2. Open it. This build is not notarised: if macOS refuses it, right-click\n"
            "   ReCore.app, choose Open, and confirm.\n"
            "3. Link Discord with the QR code Discord's phone app scans.\n\n"
            "ReCore keeps its data in ~/Library/Application Support/io.github.leriart.ReCore.\n"
            "The themes folder is the themes folder in that data folder.\n\n"
            "Third-party clients are against Discord's Terms of Service. Use ReCore only\n"
            "if losing the account would not be a disaster.\n",
            encoding="utf-8",
        )
        build = {
            "version": version,
            "flavour": args.flavour,
            "target": args.target,
            "architecture": arch,
            "cargo_features": " ".join(flavour["features"]),
            "rustc": subprocess.check_output(["rustc", "--version"], text=True, timeout=10).strip(),
            "cargo_lock_sha256": digest(ROOT / "Cargo.lock"),
            "executable_sha256": digest(binary),
            "source_archive_sha256": digest(source_path),
            "bundle_id": "io.github.leriart.ReCore",
            "themes": sorted(path.name for path in (ROOT / "themes").glob("*.theme.css")),
        }
        (resources / "BUILD.json").write_text(json.dumps(build, indent=2) + "\n", encoding="utf-8")
        if platform.system() == "Darwin":
            subprocess.run(
                ["codesign", "--force", "--deep", "--sign", "-", str(app)],
                check=False,
                timeout=120,
            )
        zip_directory(app, archive_path)
        dmg_path = None
        if not args.no_dmg and platform.system() == "Darwin" and shutil.which("hdiutil"):
            dmg_path = output / f"{base}.dmg"
            try:
                subprocess.run(
                    [
                        "hdiutil", "create", "-volname", "ReCore", "-srcfolder", str(app),
                        "-ov", "-format", "UDZO", str(dmg_path),
                    ],
                    check=True,
                    timeout=300,
                )
            except (OSError, subprocess.CalledProcessError) as error:
                # The ZIP is the release's required macOS asset; a DMG is an
                # extra when the runner's image tools cooperate.
                print(f"macOS DMG skipped: {error}", file=sys.stderr)
                dmg_path = None
    hashes = output / "SHA256SUMS.txt"
    artifacts = [archive_path, source_path, *([dmg_path] if dmg_path else [])]
    hashes.write_text("".join(f"{digest(path)}  {path.name}\n" for path in artifacts), encoding="utf-8")
    print(f"Packaged {archive_path} ({archive_path.stat().st_size / 1024 / 1024:.1f} MiB)")
    if dmg_path:
        print(f"Disk image: {dmg_path} ({dmg_path.stat().st_size / 1024 / 1024:.1f} MiB)")
    print(f"Source snapshot: {source_path}")
    print(f"Checksums: {hashes}")


if __name__ == "__main__":
    main()
