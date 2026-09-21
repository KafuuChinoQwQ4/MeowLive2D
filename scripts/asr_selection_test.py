"""Saved ASR selection is shared by review and audio-only training."""
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from training_transcription import local_model
import training_transcription


class SelectionTests(unittest.TestCase):
    def test_incomplete_unselected_download_keeps_the_existing_default_model(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            legacy = root / "engine/tools/asr/models/faster-whisper-large-v3"
            turbo = root / "data/models/faster-whisper-large-v3-turbo"
            legacy.mkdir(parents=True)
            turbo.mkdir(parents=True)
            for name in ("model.bin", "config.json", "tokenizer.json", "preprocessor_config.json"):
                (legacy / name).write_text("{}")
            with patch.dict(os.environ, {"MEOWLIVE_ASR_SELECTION": str(root / "absent.json"), "MEOWLIVE_ASR_MODEL": ""}), \
                    patch.object(training_transcription, "__file__", str(root / "scripts/training_transcription.py")):
                self.assertEqual(local_model(root / "engine"), legacy)

    def test_saved_turbo_selection_precedes_legacy_configuration_and_fails_closed(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            model = root / "turbo"
            model.mkdir()
            for name in ("model.bin", "config.json", "tokenizer.json", "preprocessor_config.json"):
                (model / name).write_text("{}")
            selection = root / "selection.json"
            selection.write_text(json.dumps({"schema": 1, "id": "test", "model_id": "faster-whisper-large-v3-turbo", "path": str(model)}))
            with patch.dict(os.environ, {"MEOWLIVE_ASR_SELECTION": str(selection)}):
                self.assertEqual(local_model(root, root / "old-model"), model)
                (model / "tokenizer.json").unlink()
                with self.assertRaisesRegex(ValueError, "tokenizer.json"):
                    local_model(root)
                selection.write_text('{"schema": 1, "model_id": "sensevoice-small", "path": "/tmp/other"}')
                with self.assertRaisesRegex(ValueError, "选择"):
                    local_model(root)

    def test_without_selection_explicit_model_remains_authoritative(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with patch.dict(os.environ, {"MEOWLIVE_ASR_SELECTION": str(root / "absent.json")}):
                with self.assertRaisesRegex(ValueError, "tokenizer.json|model.bin"):
                    local_model(root, root / "explicit-missing")


if __name__ == "__main__":
    unittest.main()
