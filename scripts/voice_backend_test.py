import importlib.util
import tempfile
import subprocess
import sys
import unittest.mock as mock
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('voice_backend', Path(__file__).with_name('voice-backend.py'))
backend = importlib.util.module_from_spec(spec)
if spec.loader and Path(spec.origin).exists():
    spec.loader.exec_module(backend)

class BackendTests(unittest.TestCase):
    def test_packaged_catalog_matches_complete_launcher_catalog(self):
        import json
        scripts = Path(__file__).resolve().parent
        exported = subprocess.check_output([
            'node', '--input-type=module', '-e',
            "import { MODEL_CATALOG } from './scripts/launcher/model-catalog.mjs'; console.log(JSON.stringify(MODEL_CATALOG));",
        ], cwd=scripts.parent, text=True)
        self.assertEqual(backend.CATALOG, json.loads(exported))

    def test_all_catalog_entries_are_probed_with_supported_capabilities_only(self):
        with tempfile.TemporaryDirectory() as directory, mock.patch.object(backend, 'ROOT', Path(directory)), \
             mock.patch.object(backend, 'LEGACY_ENGINE', Path(directory) / 'missing'):
            state = backend.probe()
        supported = {'gpt-sovits-v2': 'training_inference',
                     'faster-whisper-large-v3-turbo': 'transcription',
                     'faster-whisper-large-v3': 'transcription'}
        for item in state['models']:
            self.assertEqual(item['capability'], supported.get(item['id'], 'download_only'))
            self.assertFalse(item['downloaded'])
        self.assertIn('qwen3-asr-1.7b', {item['id'] for item in state['models']})
        self.assertIn('cosyvoice-3', {item['id'] for item in state['models']})

    def test_gpt_versions_require_extracted_g2pw_not_just_archive(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for identifier in ['gpt-sovits-v2', 'gpt-sovits-v3', 'gpt-sovits-v4']:
                item = {'id': identifier, 'markers': ['weights.pth', 'G2PWModel.zip']}
                (root / 'weights.pth').write_bytes(b'data')
                (root / 'G2PWModel.zip').write_bytes(b'archive')
                self.assertFalse(backend.downloaded(item, root), identifier)
                for name in ['g2pW.onnx', 'config.py']:
                    path = root / 'GPT_SoVITS/text/G2PWModel' / name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_bytes(b'data')
                (root / 'G2PWModel.zip').unlink()
                self.assertTrue(backend.downloaded(item, root), identifier)
                for name in ['g2pW.onnx', 'config.py']:
                    (root / 'GPT_SoVITS/text/G2PWModel' / name).unlink()

    def test_catalog_rejects_arbitrary_model_ids(self):
        self.assertTrue(hasattr(backend, 'model'), 'curated model lookup required')
        with self.assertRaises(ValueError):
            backend.model('../../etc')

    def test_existing_project_installation_is_discovered(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'GPT-SoVITS'
            root.mkdir()
            python = Path(directory) / 'python'
            python.write_text('')
            with mock.patch.object(backend, 'ROOT', root / 'managed'), \
                 mock.patch.object(backend, 'LEGACY_ENGINE', root), \
                 mock.patch.object(backend, 'LEGACY_PYTHON', python):
                self.assertEqual(backend.installation(), (root, python, False))

    def test_unpacked_g2pw_does_not_require_download_archive(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            item = {'id': 'gpt-sovits-v2', 'markers': ['weights.pth', 'G2PWModel.zip']}
            for name in ['weights.pth', 'GPT_SoVITS/text/G2PWModel/g2pW.onnx', 'GPT_SoVITS/text/G2PWModel/config.py']:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b'content')
            self.assertTrue(backend.downloaded(item, root))
            (root / 'GPT_SoVITS/text/G2PWModel/g2pW.onnx').unlink()
            self.assertFalse(backend.downloaded(item, root))

    def test_managed_probe_checks_engine_and_models_in_their_actual_directories(self):
        import json
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'backend.json').write_text(json.dumps({'schema': 1, 'revision': backend.REVISION}))
            (root / 'engine').mkdir()
            (root / 'engine/api_v2.py').write_text('# engine')
            for name in ['GPT_SoVITS/s1_train.py', 'GPT_SoVITS/s2_train.py']:
                path = root / 'engine' / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('# train')
            model_file = root / 'models/gpt-sovits-v2/weights.bin'
            model_file.parent.mkdir(parents=True)
            model_file.write_bytes(b'weights')
            for name in ['g2pW.onnx', 'config.py']:
                path = root / 'models/gpt-sovits-v2/GPT_SoVITS/text/G2PWModel' / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b'data')
            with mock.patch.object(backend, 'ROOT', root), \
                 mock.patch.object(backend, 'CATALOG', [{'id': 'gpt-sovits-v2', 'name': 'sample', 'compatibility': 'ready', 'markers': ['weights.bin']}]), \
                 mock.patch.object(backend.subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, 'true', '')), \
                 mock.patch.object(backend.subprocess, 'check_output', return_value=backend.REVISION):
                result = backend.probe()
            self.assertTrue(result['backend']['ready'], result['backend']['detail'])
            self.assertTrue(result['models'][0]['downloaded'])

    def test_existing_models_are_reused_but_downloads_stay_in_managed_storage(self):
        with tempfile.TemporaryDirectory() as directory:
            engine = Path(directory) / 'external'
            engine.mkdir()
            item = {'id': 'gpt-sovits-v2', 'markers': ['weights.pth']}
            self.assertEqual(backend.model_directory(item, engine, False), backend.ROOT / 'models/gpt-sovits-v2')
            for name in ['weights.pth', 'GPT_SoVITS/text/G2PWModel/g2pW.onnx', 'GPT_SoVITS/text/G2PWModel/config.py']:
                path = engine / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b'data')
            self.assertEqual(backend.model_directory(item, engine, False), engine)
            self.assertEqual(backend.model_directory({'id': 'whisper'}, engine, False), backend.ROOT / 'models/whisper')

    def test_existing_probe_uses_registered_environment_and_reports_dependency_failure(self):
        import json
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            engine = root / 'existing/engine'
            for name in ['api_v2.py', 'GPT_SoVITS/s1_train.py', 'GPT_SoVITS/s2_train.py']:
                path = engine / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('# code')
            python = root / 'env/bin/python'
            (root / 'existing.json').write_text(json.dumps({'engine': str(engine), 'python': str(python)}))
            with mock.patch.object(backend, 'ROOT', root), \
                 mock.patch.object(backend.subprocess, 'run', return_value=subprocess.CompletedProcess([], 1, '', 'missing dependency')):
                result = backend.probe()
            self.assertFalse(result['backend']['ready'])
            self.assertIn('missing dependency', result['backend']['detail'])
            self.assertEqual(result['backend']['pythonPath'], str(python))
            self.assertEqual(result['backend']['engineRoot'], str(engine))

    def test_cancelled_probe_stops_dependency_child_before_registering_environment(self):
        import json
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            engine = root / 'engine'
            for name in ['api_v2.py', 'GPT_SoVITS/s1_train.py', 'GPT_SoVITS/s2_train.py']:
                path = engine / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('# engine')
            marker = root / 'dependency-started'
            python = root / 'slow-python'
            python.write_text('#!' + sys.executable + '\nimport pathlib,time\npathlib.Path(' + repr(str(marker)) + ').touch()\ntime.sleep(60)\n')
            python.chmod(0o755)
            script = str(Path(spec.origin).resolve())
            code = "import importlib.util,sys,pathlib; s=importlib.util.spec_from_file_location('backend',sys.argv[1]); m=importlib.util.module_from_spec(s); s.loader.exec_module(m); m.ROOT=pathlib.Path(sys.argv[2]); sys.argv=['backend','probe','--existing-python',sys.argv[3],'--existing-engine',sys.argv[4]]; m.main()"
            child = subprocess.Popen([sys.executable, '-c', code, script, str(root), str(python), str(engine)])
            try:
                for _ in range(200):
                    if marker.exists(): break
                    backend.time.sleep(.01)
                self.assertTrue(marker.exists())
                backend.stop(root / 'runtime/operation.pid')
                self.assertIsNotNone(child.wait(timeout=3))
                self.assertFalse((root / 'existing.json').exists())
            finally:
                if child.poll() is None:
                    __import__('os').killpg(child.pid, 9)
                child.wait()

    def test_running_inference_blocks_replacing_existing_registration(self):
        import json
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original = json.dumps({'engine': '/old/engine', 'python': '/old/python'})
            (root / 'existing.json').write_text(original)
            with mock.patch.object(backend, 'ROOT', root), mock.patch.object(backend, 'running', return_value=True), \
                 mock.patch.object(backend, 'record_process'), mock.patch.object(backend.os, 'setsid'), \
                 mock.patch.object(sys, 'argv', ['backend', 'probe', '--existing-python', '/new/python', '--existing-engine', '/new/engine']):
                with self.assertRaisesRegex(ValueError, '停止语音引擎'):
                    backend.main()
            self.assertEqual((root / 'existing.json').read_text(), original)

    def test_selection_requires_complete_files(self):
        self.assertTrue(hasattr(backend, 'downloaded'), 'complete marker validation required')
        with tempfile.TemporaryDirectory() as directory:
            item = {'markers': ['config.json', 'model.bin'], 'id': 'test'}
            root = Path(directory)
            (root / 'config.json').write_text('{}')
            self.assertFalse(backend.downloaded(item, root))
            (root / 'model.bin').write_bytes(b'weights')
            self.assertTrue(backend.downloaded(item, root))

    def test_stop_terminates_owned_process_group(self):
        with tempfile.TemporaryDirectory() as directory:
            child = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(60)'], start_new_session=True)
            path = Path(directory) / 'process.json'
            try:
                backend.record_process(path, child.pid)
                backend.stop(path)
                self.assertIsNotNone(child.wait(timeout=3))
                self.assertFalse(path.exists())
            finally:
                child.kill() if child.poll() is None else None
                child.wait()

    def test_stop_kills_stubborn_descendant_after_leader_exits(self):
        import os
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'process.json'
            ready = Path(directory) / 'child.pid'
            child_code = "import os,signal,time,pathlib; signal.signal(signal.SIGTERM,signal.SIG_IGN); pathlib.Path(" + repr(str(ready)) + ").write_text(str(os.getpid())); time.sleep(60)"
            leader_code = "import subprocess,sys,time; subprocess.Popen([sys.executable,'-c'," + repr(child_code) + "]); time.sleep(60)"
            child = subprocess.Popen([sys.executable, '-c', leader_code], start_new_session=True)
            try:
                for _ in range(100):
                    if ready.exists(): break
                    backend.time.sleep(.01)
                self.assertTrue(ready.exists())
                backend.record_process(path, child.pid)
                backend.stop(path)
                child.wait(timeout=3)
                pid = int(ready.read_text())
                if Path(f'/proc/{pid}/stat').exists():
                    self.assertEqual(Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()[0], 'Z')
            finally:
                try: os.killpg(child.pid, 9)
                except ProcessLookupError: pass
                child.wait()

    def test_existing_unloaded_inference_is_enabled_and_waited(self):
        self.assertTrue(hasattr(backend, 'ensure_loaded'))
        with mock.patch.object(backend, 'lifecycle', side_effect=[{'state': 'unloaded'}, {'state': 'loading'}, {'state': 'loaded'}]) as call, mock.patch.object(backend.time, 'sleep'):
            backend.ensure_loaded()
            self.assertEqual(call.call_args_list[1], mock.call(True))

    def test_process_record_from_another_boot_is_not_owned(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'process.json'
            backend.record_process(path, __import__('os').getpid())
            record = __import__('json').loads(path.read_text())
            record['boot'] = 'previous boot'
            self.assertFalse(backend.same_process(record))

    def test_pid_identity_prevents_killing_reused_pid(self):
        self.assertTrue(hasattr(backend, 'same_process'), 'process identity validation required')
        self.assertFalse(backend.same_process({'pid': 1, 'start': 'invalid'}))

if __name__ == '__main__':
    unittest.main()
