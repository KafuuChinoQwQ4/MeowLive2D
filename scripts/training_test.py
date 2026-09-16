"""Runner boundary tests require only the Python standard library."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import uuid
import wave
import zipfile
from engine_workspace import environment, configure_low_memory
archive_spec = importlib.util.spec_from_file_location("model_archive", Path(__file__).with_name("extract-model-archive.py"))
model_archive = importlib.util.module_from_spec(archive_spec)
archive_spec.loader.exec_module(model_archive)

spec = importlib.util.spec_from_file_location("runner", Path(__file__).with_name("train-gpt-sovits.py"))
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class TrainingTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # npm check runs these tests before Cargo creates its output directory.
        (Path(__file__).resolve().parents[1] / "target").mkdir(exist_ok=True)

    def job(self, root):
        (root / "clips").mkdir()
        clips = []
        for index in range(2):
            path = f"clips/{index:03}.wav"
            with wave.open(str(root / path), "wb") as out:
                out.setparams((1, 2, 8000, 0, "NONE", "none"))
                out.writeframes(b"\x01\x00" * 24000)
            clips.append({"path": path, "language": "zh", "text": "已校对文本"})
        value = {"schema": 1, "id": str(uuid.uuid4()), "model_version": "v2", "batch_size": 1, "fp16": True,
                 "sovits_epochs": 1, "gpt_epochs": 1, "clips": clips,
                 "artifacts": {"gpt": "artifacts/gpt.ckpt", "sovits": "artifacts/sovits.pth"}}
        (root / "job.json").write_text(json.dumps(value))
        return value

    def test_manifest_rejects_unreviewable_text_paths_and_unbounded_parameters(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "target") as temp:
            root = Path(temp)
            value = self.job(root)
            self.assertEqual(len(runner.load_job(root / "job.json")[1]["clips"]), 2)
            for mutate in [lambda v: v.update(gpt_epochs=21), lambda v: v.update(fp16=False),
                           lambda v: v["clips"][0].update(text="one|two"),
                           lambda v: v["clips"][0].update(path="../outside.wav")]:
                changed = json.loads(json.dumps(value))
                mutate(changed)
                (root / "job.json").write_text(json.dumps(changed))
                with self.assertRaises(ValueError):
                    runner.load_job(root / "job.json")

    def test_offline_children_do_not_inherit_proxy_or_analytics(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "target") as temp:
            with patch.dict(os.environ, {"ALL_PROXY": "socks5://invalid:9999", "https_proxy": "http://invalid:1"}):
                result = environment(Path(temp), {"bert": Path(temp)})
                self.assertFalse("ALL_PROXY" in result)
                self.assertFalse("https_proxy" in result)
                self.assertEqual(result["GRADIO_ANALYTICS_ENABLED"], "False")
                self.assertEqual(result["HF_HUB_OFFLINE"], "1")

    def test_low_memory_override_is_scoped_and_fails_for_unknown_upstream(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "target") as temp:
            root = Path(temp)
            (root / "GPT_SoVITS").mkdir()
            source = root / "GPT_SoVITS/s2_train.py"
            source.write_text("        eps=hps.train.eps,\n" * 2)
            configure_low_memory(root)
            self.assertEqual(source.read_text().count("foreach=False"), 2)
            configure_low_memory(root)
            source.write_text("changed upstream")
            with self.assertRaises(ValueError):
                configure_low_memory(root)

    def test_model_archive_extracts_only_expected_safe_members(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            bundle = root / "model.zip"
            with zipfile.ZipFile(bundle, "w") as archive:
                archive.writestr("G2PWModel/config.py", "configured")
            model_archive.extract(bundle, root / "output")
            self.assertEqual((root / "output/G2PWModel/config.py").read_text(), "configured")
            for name in ["../escape", "/absolute", "G2PWModel/../../escape", "unexpected/file"]:
                with zipfile.ZipFile(bundle, "w") as archive:
                    archive.writestr(name, "bad")
                with self.assertRaises(ValueError):
                    model_archive.extract(bundle, root / "output")
            self.assertFalse((root / "escape").exists())

    def test_symlink_clip_is_rejected(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "target") as temp:
            root = Path(temp)
            self.job(root)
            (root / "clips/000.wav").unlink()
            (root / "clips/000.wav").symlink_to(root / "clips/001.wav")
            with self.assertRaises(ValueError):
                runner.load_job(root / "job.json")


if __name__ == "__main__":
    unittest.main()
