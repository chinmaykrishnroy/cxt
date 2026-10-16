"""Prepare WinGet manifests from a published stable cxt release; never submit."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import urllib.error
import urllib.parse
import urllib.request

REPO = "chinmaykrishnroy/cxt"
PACKAGE = "ChinmayKrishnRoy.cxt"
ARCHES = {"x64": "x86_64", "arm64": "aarch64"}


def api(path):
    headers = {"Accept": "application/vnd.github+json", "User-Agent": "cxt-winget"}
    if os.environ.get("GH_TOKEN"):
        headers["Authorization"] = "Bearer " + os.environ["GH_TOKEN"]
    request = urllib.request.Request("https://api.github.com/" + path, headers=headers)
    with urllib.request.urlopen(request, timeout=60) as response:
        return json.load(response)


def stable_version(tag):
    if not re.fullmatch(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", tag):
        raise ValueError("Only stable vMAJOR.MINOR.PATCH tags are supported")
    return tag[1:]


def existing_submission(version):
    path = f"manifests/c/ChinmayKrishnRoy/cxt/{version}"
    try:
        api(f"repos/microsoft/winget-pkgs/contents/{path}")
        return "Version already exists in microsoft/winget-pkgs"
    except urllib.error.HTTPError as error:
        if error.code != 404:
            raise
    # Search exact changed paths, not loose title matches such as 0.5.20.
    query = urllib.parse.quote(f"repo:microsoft/winget-pkgs is:pr is:open {PACKAGE}")
    search = api(f"search/issues?q={query}&per_page=100")
    if search.get("incomplete_results") or search["total_count"] > 100:
        raise ValueError("Cannot safely establish absence of duplicate PRs")
    for issue in search["items"]:
        files = api(f"repos/microsoft/winget-pkgs/pulls/{issue['number']}/files?per_page=100")
        if len(files) == 100:
            raise ValueError("Unexpectedly large package PR; check duplicates manually")
        if any(file["filename"].startswith(path + "/") for file in files):
            return "Existing PR: " + issue["html_url"]
    return None


def download(asset, destination):
    expected_url = f"https://github.com/{REPO}/releases/download/"
    if not asset["browser_download_url"].startswith(expected_url):
        raise ValueError("Unexpected asset origin")
    with urllib.request.urlopen(asset["browser_download_url"], timeout=60) as response:
        with destination.open("wb") as output:
            while chunk := response.read(1024 * 1024):
                output.write(chunk)
    return hashlib.sha256(destination.read_bytes()).hexdigest().upper()


def manifests(version, hashes):
    common = f"PackageIdentifier: {PACKAGE}\nPackageVersion: {version}\n"
    footer = "ManifestVersion: 1.12.0\n"
    installer = common + "InstallerType: portable\nCommands:\n- cxt\nInstallers:\n"
    for arch, triple in ARCHES.items():
        installer += (
            f"- Architecture: {arch}\n"
            f"  InstallerUrl: https://github.com/{REPO}/releases/download/v{version}/cxt-{triple}-pc-windows-msvc.exe\n"
            f"  InstallerSha256: {hashes[arch]}\n"
            f"  Dependencies:\n    PackageDependencies:\n    - PackageIdentifier: Microsoft.VCRedist.2015+.{arch}\n"
        )
    locale = common + (
        "PackageLocale: en-US\nPublisher: Chinmay Krishn Roy\n"
        "PublisherUrl: https://github.com/chinmaykrishnroy\n"
        f"PublisherSupportUrl: https://github.com/{REPO}/issues\n"
        f"PackageName: cxt\nPackageUrl: https://github.com/{REPO}\nLicense: MIT\n"
        f"LicenseUrl: https://github.com/{REPO}/blob/v{version}/LICENSE\n"
        "ShortDescription: Pack relevant codebase files and dependencies into clean Markdown context for AI-assisted development.\n"
        f"ReleaseNotesUrl: https://github.com/{REPO}/releases/tag/v{version}\n"
    )
    values = {
        "installer": installer + "ManifestType: installer\n" + footer,
        "defaultLocale": locale + "ManifestType: defaultLocale\n" + footer,
        "version": common + "DefaultLocale: en-US\nManifestType: version\n" + footer,
    }
    suffixes = {"installer": ".installer", "defaultLocale": ".locale.en-US", "version": ""}
    return {
        PACKAGE + suffixes[kind] + ".yaml":
        f"# yaml-language-server: $schema=https://aka.ms/winget-manifest.{kind}.1.12.0.schema.json\n" + body
        for kind, body in values.items()
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    version = stable_version(args.tag)
    release = api(f"repos/{REPO}/releases/tags/{args.tag}")
    if release["draft"] or release["prerelease"] or release["tag_name"] != args.tag:
        raise ValueError("Release must be published and stable")
    duplicate = existing_submission(version)
    if duplicate:
        print(duplicate)
        return
    assets = {asset["name"]: asset for asset in release["assets"]}
    args.out.mkdir(parents=True, exist_ok=False)
    downloads = args.out / "downloads"
    downloads.mkdir()
    sums_path = downloads / "SHA256SUMS.txt"
    download(assets["SHA256SUMS.txt"], sums_path)
    sums = {}
    for line in sums_path.read_text(encoding="utf-8").splitlines():
        digest, name = line.split(maxsplit=1)
        name = name.lstrip("*")
        if name in sums or not re.fullmatch(r"[a-fA-F0-9]{64}", digest):
            raise ValueError("Invalid or duplicate checksum entry")
        sums[name] = digest.upper()
    hashes = {}
    for arch, triple in ARCHES.items():
        name = f"cxt-{triple}-pc-windows-msvc.exe"
        digest = download(assets[name], downloads / name)
        if digest != sums[name]:
            raise ValueError("Release checksum mismatch: " + name)
        recorded = assets[name].get("digest")
        if recorded and recorded.lower() != "sha256:" + digest.lower():
            raise ValueError("GitHub asset digest mismatch: " + name)
        hashes[arch] = digest
    manifest_dir = args.out / "manifests"
    manifest_dir.mkdir()
    for name, body in manifests(version, hashes).items():
        (manifest_dir / name).write_text(body, encoding="utf-8")
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
            output.write("ready=true\n")
    print("Verified both Windows downloads and prepared manifests")


if __name__ == "__main__":
    main()
