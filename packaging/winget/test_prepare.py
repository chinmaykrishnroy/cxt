"""Offline regression tests; no package installs or PR submissions."""
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import urllib.error

import prepare


class PrepareTests(unittest.TestCase):
    def test_stable_tags_only(self):
        self.assertEqual(prepare.stable_version("v0.5.2"), "0.5.2")
        for tag in ["v0.5.2-dev.1", "0.5.2", "v00.5.2", "v0.5.2;echo bad", "v0.5.2\n"]:
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                prepare.stable_version(tag)

    def test_both_architectures_and_runtime_dependencies(self):
        files = prepare.manifests("0.5.2", {"x64": "A" * 64, "arm64": "B" * 64})
        self.assertEqual(len(files), 3)
        installer = files[prepare.PACKAGE + ".installer.yaml"]
        self.assertIn("Commands:\n- cxt\n", installer)
        self.assertNotIn("Scope:", installer)
        for arch in prepare.ARCHES:
            self.assertIn("Architecture: " + arch, installer)
            self.assertIn("Microsoft.VCRedist.2015+." + arch, installer)

    def test_catalog_version_skips(self):
        with patch.object(prepare, "api", return_value=[]) as api:
            self.assertIn("already exists", prepare.existing_submission("0.5.2"))
            self.assertEqual(api.call_count, 1)

    def test_exact_pr_paths_not_similar_version_titles(self):
        not_found = urllib.error.HTTPError("url", 404, "missing", {}, None)
        responses = [not_found, {"total_count": 1, "items": [{"number": 7, "html_url": "PR7"}]},
                     [{"filename": "manifests/c/ChinmayKrishnRoy/cxt/0.5.20/file.yaml"}]]
        with patch.object(prepare, "api", side_effect=responses):
            self.assertIsNone(prepare.existing_submission("0.5.2"))
        responses[-1] = [{"filename": "manifests/c/ChinmayKrishnRoy/cxt/0.5.2/file.yaml"}]
        with patch.object(prepare, "api", side_effect=responses):
            self.assertEqual(prepare.existing_submission("0.5.2"), "Existing PR: PR7")

    def test_api_errors_do_not_mean_absent(self):
        forbidden = urllib.error.HTTPError("url", 403, "forbidden", {}, None)
        with patch.object(prepare, "api", side_effect=forbidden), self.assertRaises(urllib.error.HTTPError):
            prepare.existing_submission("0.5.2")

    def test_duplicate_exits_before_download_or_output(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "not-created"
            with patch("sys.argv", ["prepare.py", "--tag", "v0.5.2", "--out", str(output)]), \
                 patch.object(prepare, "api", return_value={"draft": False, "prerelease": False, "tag_name": "v0.5.2"}), \
                 patch.object(prepare, "existing_submission", return_value="Existing PR"), \
                 patch.object(prepare, "download") as download:
                prepare.main()
                download.assert_not_called()
                self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
