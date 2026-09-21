"""Runner boundary tests require only the Python standard library."""
from contextlib import redirect_stderr, redirect_stdout
import importlib.util
import hashlib
import io
import json
import os
import signal
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import time
import types
import unittest
from unittest.mock import patch
import uuid
import wave
import zipfile
from engine_workspace import environment, configure_low_memory
import engine_workspace
archive_spec = importlib.util.spec_from_file_location("model_archive", Path(__file__).with_name("extract-model-archive.py"))
model_archive = importlib.util.module_from_spec(archive_spec)
archive_spec.loader.exec_module(model_archive)

spec = importlib.util.spec_from_file_location("runner", Path(__file__).with_name("train-gpt-sovits.py"))
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


FAKE_WHISPER = '''
import json
import os
from pathlib import Path
from types import SimpleNamespace
print("fake dependency import diagnostic")
def record(event):
    with Path(os.environ["FAKE_ASR_LOG"]).open("a") as out:
        out.write(json.dumps(event) + "\\n")
class WhisperModel:
    def __init__(self, path, **kwargs):
        print("fake model diagnostic")
        record({"event": "load", "path": path, **kwargs})
        self.supported_languages = ["zh", "en", "ja", "ko", "yue"]
        self.model = SimpleNamespace(unload_model=lambda: record({"event": "unload"}))
        self.hf_tokenizer = SimpleNamespace(token_to_id=lambda token:
            None if token == "<|yue|>" and os.environ.get("FAKE_ASR_NO_YUE") else 1)
    def transcribe(self, audio, **kwargs):
        print("fake recognition diagnostic")
        record({"event": "transcribe", "audio": audio, **kwargs})
        text = os.environ.get("FAKE_ASR_TEXT", " 自动提取的文本 ")
        return iter([SimpleNamespace(text=text)]), SimpleNamespace(language=kwargs["language"])
'''


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
                           lambda v: v["clips"][0].update(text="隐藏\x7f控制字符"),
                           lambda v: v["clips"][0].update(path="../outside.wav")]:
                changed = json.loads(json.dumps(value))
                mutate(changed)
                (root / "job.json").write_text(json.dumps(changed))
                with self.assertRaises(ValueError):
                    runner.load_job(root / "job.json")

    def test_manifest_defaults_legacy_performance_and_validates_explicit_bounds(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "target") as temp:
            root = Path(temp)
            legacy = self.job(root)
            loaded = runner.load_job(root / "job.json")[1]
            self.assertEqual(loaded["performance"], {
                "batch_size": 1, "data_workers": 1, "cpu_threads": 2,
                "gpu_index": 0, "low_memory": True,
            })
            valid = json.loads(json.dumps(legacy))
            valid.pop("batch_size")
            valid["performance"] = {
                "batch_size": 16, "data_workers": 0, "cpu_threads": 16,
                "gpu_index": 15, "low_memory": False,
            }
            (root / "job.json").write_text(json.dumps(valid))
            self.assertEqual(runner.load_job(root / "job.json")[1]["performance"], valid["performance"])
            for key, value in [("batch_size", 0), ("batch_size", 17), ("data_workers", 9),
                               ("cpu_threads", 0), ("cpu_threads", 17), ("gpu_index", 16),
                               ("low_memory", 1)]:
                changed = json.loads(json.dumps(valid))
                changed["performance"][key] = value
                (root / "job.json").write_text(json.dumps(changed))
                with self.assertRaises(ValueError):
                    runner.load_job(root / "job.json")

    def test_configurations_apply_performance_to_both_trainers_and_zero_workers(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "target") as temp:
            root = Path(temp)
            work = root / "work"
            configs = work / "GPT_SoVITS/configs"
            configs.mkdir(parents=True)
            (configs / "s2.json").write_text(json.dumps({"train": {}, "model": {}, "data": {}}))
            (configs / "s1longer-v2.yaml").write_text("train: {}\ndata: {}\n")
            job = self.job(root)
            job["performance"] = {
                "batch_size": 4, "data_workers": 0, "cpu_threads": 8,
                "gpu_index": 3, "low_memory": False,
            }
            models = {name: root / name for name in ("gpt", "sovits", "discriminator")}
            fake_yaml = types.SimpleNamespace(
                safe_load=lambda value: json.loads(value),
                safe_dump=lambda value, **_: json.dumps(value),
            )
            (configs / "s1longer-v2.yaml").write_text(json.dumps({"train": {}, "data": {}}))
            with patch.dict(sys.modules, {"yaml": fake_yaml}):
                runner.configurations(root, job, work, models)
            s2 = json.loads((root / "s2.json").read_text())
            s1 = json.loads((root / "s1.yaml").read_text())
            for config in (s1, s2):
                self.assertEqual(config["train"]["batch_size"], 4)
                self.assertEqual(config["data"]["num_workers"], 0)
                self.assertIs(config["data"]["persistent_workers"], False)
                self.assertIsNone(config["data"]["prefetch_factor"])
            env = environment(work, {"bert": root}, job["performance"])
            self.assertEqual(env["OMP_NUM_THREADS"], "8")
            self.assertEqual(env["CUDA_VISIBLE_DEVICES"], "3")
            self.assertEqual(env["_CUDA_VISIBLE_DEVICES"], "3")
            # s2_train overwrites CUDA_VISIBLE_DEVICES from gpu_numbers before
            # importing torch, so this must be the physical selected device.
            self.assertEqual(s2["train"]["gpu_numbers"], env["CUDA_VISIBLE_DEVICES"])

            # A later round must initialize both trainers from this voice's
            # previous pair, while retaining the generic discriminator.
            (root / "base").mkdir()
            base = {"job_id": str(uuid.uuid4()), "voice_id": "witch", "sha256": {}}
            job["voice_id"] = "witch"
            for key, filename in (("gpt", "gpt.ckpt"), ("sovits", "sovits.pth")):
                base[key] = f"base/{filename}"
                content = f"previous {key} weights".encode()
                (root / base[key]).write_bytes(content)
                base["sha256"][key] = hashlib.sha256(content).hexdigest()
            job["base"] = base
            with patch.dict(sys.modules, {"yaml": fake_yaml}):
                runner.configurations(root, job, work, models)
            s2 = json.loads((root / "s2.json").read_text())
            s1 = json.loads((root / "s1.yaml").read_text())
            self.assertEqual(s2["train"]["pretrained_s2G"], str(root / "base/sovits.pth"))
            self.assertEqual(s1["pretrained_s1"], str(root / "base/gpt.ckpt"))
            self.assertEqual(s2["train"]["pretrained_s2D"], str(models["discriminator"]))
            self.assertEqual(models["sovits"], root / "sovits")
            (root / "job.json").write_text(json.dumps(job))
            self.assertEqual(runner.load_job(root / "job.json")[1]["base"], base)
            for invalid in ["voice", "path", "hash", "missing"]:
                changed = json.loads(json.dumps(job))
                if invalid == "voice": changed["base"]["voice_id"] = "other"
                elif invalid == "path": changed["base"]["gpt"] = "../gpt.ckpt"
                elif invalid == "hash": changed["base"]["sha256"]["gpt"] = "0" * 64
                else: (root / "base/gpt.ckpt").unlink()
                (root / "job.json").write_text(json.dumps(changed))
                with self.assertRaises((ValueError, OSError)):
                    runner.load_job(root / "job.json")

    def test_owned_trainers_apply_zero_worker_loader_settings(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "target") as temp:
            work = Path(temp)
            s2 = work / "GPT_SoVITS/s2_train.py"
            gpt = work / "GPT_SoVITS/AR/data/data_module.py"
            s2.parent.mkdir(parents=True)
            gpt.parent.mkdir(parents=True)
            s2.write_text('''captured = {}
class Data:
    num_workers = 0
    persistent_workers = False
    prefetch_factor = None
class Hps:
    data = Data()
hps = Hps()
def DataLoader(dataset, **kwargs):
    captured.update(kwargs)
train_dataset = object()
train_sampler = object()
collate_fn = object()
train_loader = DataLoader(
        train_dataset,
        num_workers=5,
        shuffle=False,
        pin_memory=True,
        collate_fn=collate_fn,
        batch_sampler=train_sampler,
        persistent_workers=True,
        prefetch_factor=3,
    )
''')
            gpt.write_text('''captured = {}
def DataLoader(dataset, **kwargs):
    captured.update(kwargs)
class Loader:
    def __init__(self):
        self._train_dataset = type("Dataset", (), {"collate": object()})()
        self.num_workers = 0
        self.config = {"data": {"persistent_workers": False, "prefetch_factor": None}}
    def run(self):
        batch_size = 1
        sampler = object()
        return DataLoader(
            self._train_dataset,
            batch_size=batch_size,
            sampler=sampler,
            collate_fn=self._train_dataset.collate,
            num_workers=self.num_workers,
            persistent_workers=True,
            prefetch_factor=16,
        )
''')
            engine_workspace.configure_training_loaders(work)
            s2_scope = {}
            exec(s2.read_text(), s2_scope)
            gpt_scope = {}
            exec(gpt.read_text(), gpt_scope)
            gpt_scope["Loader"]().run()
            for captured in (s2_scope["captured"], gpt_scope["captured"]):
                self.assertEqual(captured["num_workers"], 0)
                self.assertIs(captured["persistent_workers"], False)
                self.assertIsNone(captured["prefetch_factor"])

    def test_manifest_accepts_explicit_audio_only_and_rejects_mixed_or_unknown_modes(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "target") as temp:
            root = Path(temp)
            value = self.job(root)
            value["text_mode"] = "audio_only"
            for clip in value["clips"]:
                clip["text"] = ""
            (root / "job.json").write_text(json.dumps(value))
            self.assertEqual(runner.load_job(root / "job.json")[1]["text_mode"], "audio_only")
            for mode, texts in [("reviewed_text", ["", ""]), ("unknown", ["一", "二"]),
                                ("audio_only", ["", "手填文本"]), (None, ["", ""])]:
                changed = json.loads(json.dumps(value))
                if mode is None:
                    changed.pop("text_mode")
                else:
                    changed["text_mode"] = mode
                for clip, text in zip(changed["clips"], texts):
                    clip["text"] = text
                (root / "job.json").write_text(json.dumps(changed))
                with self.assertRaises(ValueError):
                    runner.load_job(root / "job.json")

    def asr_fixture(self, root):
        self.job(root)
        model = root / "engine/tools/asr/models/faster-whisper-large-v3"
        model.mkdir(parents=True)
        for name in ("model.bin", "config.json", "tokenizer.json", "preprocessor_config.json"):
            (model / name).write_text("{}")
        fake = root / "fake"
        fake.mkdir()
        (fake / "faster_whisper.py").write_text(FAKE_WHISPER)
        env = {**os.environ, "PYTHONPATH": str(fake), "FAKE_ASR_LOG": str(root / "asr.jsonl"),
               "MEOWLIVE_ASR_SELECTION": str(root / "selection.json")}
        env["MEOWLIVE_ASR_MODEL"] = str(model)
        return model, env

    def run_asr(self, root, env, language="zh", extra=()):
        return subprocess.run([sys.executable, "-B", "-s", str(Path(__file__).with_name("transcribe-training.py")),
                               "--audio", str(root / "clips/000.wav"), "--language", language,
                               "--engine-root", str(root / "engine"), *extra],
                              text=True, capture_output=True, env=env, check=False)

    def test_asr_cli_emits_only_json_and_keeps_model_local_on_cpu(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "target") as temp:
            root = Path(temp)
            model, env = self.asr_fixture(root)
            result = self.run_asr(root, env, "yue")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout), {"text": "自动提取的文本", "language": "yue"})
            self.assertEqual(len(result.stdout.splitlines()), 1)
            self.assertIn("fake model diagnostic", result.stderr)
            events = [json.loads(line) for line in (root / "asr.jsonl").read_text().splitlines()]
            self.assertEqual(events[0]["path"], str(model))
            self.assertEqual(events[0]["device"], "cpu")
            self.assertEqual(events[0]["compute_type"], "int8")
            self.assertIs(events[0]["local_files_only"], True)
            self.assertEqual(events[-1]["event"], "unload")

    def test_asr_cli_rejects_empty_invalid_text_and_unsupported_cantonese(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "target") as temp:
            root = Path(temp)
            _, env = self.asr_fixture(root)
            for text in (" ", "one|two", "坏\n文本", "坏\x7f文本", "字" * 501):
                with self.subTest(text=repr(text[:10])):
                    result = self.run_asr(root, {**env, "FAKE_ASR_TEXT": text})
                    self.assertNotEqual(result.returncode, 0)
                    self.assertEqual(result.stdout, "")
                    self.assertIn("转写", result.stderr)
            result = self.run_asr(root, {**env, "FAKE_ASR_NO_YUE": "1"}, "yue")
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(result.stdout, "")
            self.assertIn("粤语", result.stderr)

    def test_asr_cli_reports_missing_local_model_and_rejects_symlink_cache(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "target") as temp:
            root = Path(temp)
            model, env = self.asr_fixture(root)
            (model / "tokenizer.json").unlink()
            result = self.run_asr(root, env)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("本地", result.stderr)
            self.assertIn("tokenizer.json", result.stderr)
            self.assertEqual(result.stdout, "")
            self.assertFalse((root / "asr.jsonl").exists())
            (model / "tokenizer.json").write_text("{}")
            outside = root / "unowned"
            outside.mkdir()
            (root / "clips/.asr-cache").symlink_to(outside, target_is_directory=True)
            result = self.run_asr(root, env)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(list(outside.iterdir()), [])

    def test_asr_cli_rejects_missing_preprocessor_before_loading_model(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "target") as temp:
            root = Path(temp)
            model, env = self.asr_fixture(root)
            (model / "preprocessor_config.json").unlink()
            result = self.run_asr(root, env)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("preprocessor_config.json", result.stderr)
            self.assertEqual(result.stdout, "")
            self.assertFalse((root / "asr.jsonl").exists())

    def test_runner_transcribes_one_batch_and_never_overwrites_reviewed_text(self):
        self.assertTrue(hasattr(runner, "complete_transcripts"), "训练器尚未接入自动转写")
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "target") as temp:
            root = Path(temp)
            _, env = self.asr_fixture(root)
            original = json.loads((root / "job.json").read_text())
            fake = types.ModuleType("faster_whisper")
            with patch.dict(os.environ, env), redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
                exec(FAKE_WHISPER, fake.__dict__)
                with patch.dict(sys.modules, {"faster_whisper": fake}):
                    runner.complete_transcripts(root, original, root / "engine")
                    self.assertEqual([clip["text"] for clip in original["clips"]], ["已校对文本", "已校对文本"])
                    self.assertFalse((root / "asr.jsonl").exists())
                    original["text_mode"] = "audio_only"
                    for clip in original["clips"]:
                        clip["text"] = ""
                    runner.complete_transcripts(root, original, root / "engine")
            self.assertEqual([clip["text"] for clip in original["clips"]], ["自动提取的文本", "自动提取的文本"])
            events = [json.loads(line)["event"] for line in (root / "asr.jsonl").read_text().splitlines()]
            self.assertEqual(events, ["load", "transcribe", "transcribe", "unload"])
            self.assertEqual(json.loads((root / "job.json").read_text())["clips"][0]["text"], "已校对文本")

    def test_asr_cli_model_override_precedes_environment_and_does_not_fall_back(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "target") as temp:
            root = Path(temp)
            model, env = self.asr_fixture(root)
            env["MEOWLIVE_ASR_MODEL"] = str(root / "absent")
            result = self.run_asr(root, env)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse((root / "asr.jsonl").exists())
            result = self.run_asr(root, env, extra=("--model", str(model)))
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout)["text"], "自动提取的文本")

    def test_runner_reports_transcription_failure_before_training_and_keeps_empty_inputs(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "target") as temp:
            root = Path(temp)
            model, env = self.asr_fixture(root)
            value = json.loads((root / "job.json").read_text())
            value["text_mode"] = "audio_only"
            for clip in value["clips"]:
                clip["text"] = ""
            (root / "job.json").write_text(json.dumps(value))
            (model / "model.bin").unlink()
            output = io.StringIO()
            with patch.dict(os.environ, env), redirect_stdout(output), \
                    patch.object(sys, "argv", ["runner", "--job", str(root / "job.json"),
                                               "--engine-root", str(root / "engine")]), \
                    patch.object(runner, "assets", side_effect=AssertionError("不能先加载训练模型")):
                with self.assertRaisesRegex(ValueError, "本地 ASR 模型"):
                    runner.main()
            events = [json.loads(line) for line in output.getvalue().splitlines()]
            self.assertEqual(events[-1]["state"], "failed")
            self.assertEqual(events[-1]["error"], "transcription_failed")
            self.assertEqual(json.loads((root / "job.json").read_text())["clips"][0]["text"], "")

    def test_offline_children_do_not_inherit_proxy_or_analytics(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "target") as temp:
            with patch.dict(os.environ, {"ALL_PROXY": "socks5://invalid:9999", "https_proxy": "http://invalid:1"}):
                result = environment(Path(temp), {"bert": Path(temp)})
                self.assertFalse("ALL_PROXY" in result)
                self.assertFalse("https_proxy" in result)
                self.assertEqual(result["GRADIO_ANALYTICS_ENABLED"], "False")
                self.assertEqual(result["HF_HUB_OFFLINE"], "1")

    @unittest.skipUnless(sys.platform == "linux", "训练子进程使用 Linux Unix socket")
    def test_stage_can_share_data_from_deep_job_and_cleans_temporary_directory(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "target") as temp:
            root = Path(temp)
            work = root / ("deep-job-" + "x" * 60) / "work"
            work.mkdir(parents=True)
            script = work / "stage.py"
            script.write_text('''import json, os, tempfile
from multiprocessing.connection import Listener
from pathlib import Path
# Python 3.10 multiprocessing creates its socket below TMPDIR.
with tempfile.TemporaryDirectory(prefix="pymp-", dir=tempfile.gettempdir()) as shared:
    address = tempfile.mktemp(prefix="listener-", dir=shared)
    with Listener(address=address, family="AF_UNIX") as listener:
        Path("result.json").write_text(json.dumps({"tmp": tempfile.gettempdir(), "socket": listener.address}))
''')
            runner.run_stage(script, [], work, environment(work, {"bert": work}))
            result = json.loads((work / "result.json").read_text())
            self.assertLess(len(os.fsencode(result["socket"])), 108)
            self.assertFalse(Path(result["tmp"]).exists())
            self.assertTrue(Path(result["tmp"]).is_relative_to(Path(__file__).resolve().parents[1] / "data"))

    def test_stage_flushes_small_logs_before_exit_and_cleans_up_on_failure(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "target") as temp:
            root = Path(temp)
            work = root / "work"
            work.mkdir()
            script = work / "stage.py"
            script.write_text('''import os, time
from pathlib import Path
Path("tmp.txt").write_text(os.environ["TMPDIR"])
print("first training batch", flush=True)
deadline = time.monotonic() + 5
while not Path("release").exists() and time.monotonic() < deadline:
    time.sleep(0.02)
raise SystemExit(7)
''')
            errors = []
            def execute():
                try:
                    runner.run_stage(script, [], work, environment(work, {"bert": work}))
                except Exception as error:
                    errors.append(error)
            thread = threading.Thread(target=execute)
            thread.start()
            visible = False
            try:
                deadline = time.monotonic() + 3
                while time.monotonic() < deadline:
                    log = root / "stages.log"
                    if log.exists() and b"first training batch" in log.read_bytes():
                        visible = True
                        break
                    time.sleep(0.02)
            finally:
                (work / "release").touch()
                thread.join(timeout=10)
            self.assertFalse(thread.is_alive())
            self.assertTrue(visible, "阶段运行期间的小段日志没有及时落盘")
            self.assertEqual(len(errors), 1)
            self.assertIn("退出码 7", str(errors[0]))
            self.assertFalse(Path((work / "tmp.txt").read_text()).exists())

    @unittest.skipUnless(sys.platform == "linux", "Linux 训练进程组取消")
    def test_cancelled_runner_cleans_stage_temporary_directory(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "target") as temp:
            root = Path(temp)
            work = root / "work"
            work.mkdir()
            stage = work / "stage.py"
            stage.write_text('''import os, time
from pathlib import Path
Path("temporary.txt").write_text(os.environ["TMPDIR"])
time.sleep(30)
''')
            # Keep the real main and stage lifecycle; replace model/data setup.
            bootstrap = '''import importlib.util, sys
from pathlib import Path
spec = importlib.util.spec_from_file_location("runner", sys.argv[1])
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)
work = Path(sys.argv[2])
def load_job(path):
    runner.run_stage(work / "stage.py", [], work, runner.environment(work, {"bert": work}))
    raise AssertionError("stage must be cancelled")
runner.load_job = load_job
sys.argv = ["runner", "--job", str(work.parent / "job.json"), "--engine-root", str(work)]
runner.main()
'''
            command = [sys.executable, "-c", bootstrap, str(Path(runner.__file__).resolve()), str(work)]
            env = {**os.environ, "PYTHONPATH": str(Path(runner.__file__).parent), "PYTHONDONTWRITEBYTECODE": "1"}
            process = subprocess.Popen(command, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                       start_new_session=True)
            try:
                deadline = time.monotonic() + 5
                while not (work / "temporary.txt").exists() and process.poll() is None and time.monotonic() < deadline:
                    time.sleep(0.02)
                self.assertTrue((work / "temporary.txt").exists())
                temporary = Path((work / "temporary.txt").read_text())
                os.killpg(process.pid, signal.SIGTERM)
                process.communicate(timeout=5)
                self.assertFalse(temporary.exists(), "取消任务后遗留阶段临时目录")
            finally:
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGKILL)
                process.communicate(timeout=5)

    def test_low_memory_override_is_scoped_and_fails_for_unknown_upstream(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1] / "target") as temp:
            root = Path(temp)
            (root / "GPT_SoVITS").mkdir()
            source = root / "GPT_SoVITS/s2_train.py"
            source.write_text("        eps=hps.train.eps,\n" * 2)
            configure_low_memory(root, False)
            self.assertNotIn("foreach=False", source.read_text())
            configure_low_memory(root)
            self.assertEqual(source.read_text().count("foreach=False"), 2)
            configure_low_memory(root)
            source.write_text("changed upstream")
            with self.assertRaises(ValueError):
                configure_low_memory(root)

    def test_inference_memory_mode_uses_upstream_pinyin_fallback_and_can_restore(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "GPT_SoVITS/text/chinese2.py"
            source.parent.mkdir(parents=True)
            original = "import os\n# is_g2pw = False\nis_g2pw = True  # upstream default\nif is_g2pw:\n    pass\n"
            source.write_text(original)
            engine_workspace.configure_inference_memory(root, "low")
            self.assertIn("is_g2pw = False  # upstream default", source.read_text())
            engine_workspace.configure_inference_memory(root, "low")
            engine_workspace.configure_inference_memory(root, "standard")
            self.assertEqual(source.read_text(), original)
            source.write_text("changed upstream")
            with self.assertRaises(ValueError):
                engine_workspace.configure_inference_memory(root, "low")

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
