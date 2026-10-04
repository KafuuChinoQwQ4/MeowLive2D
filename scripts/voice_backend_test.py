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
    def test_catalog_rejects_arbitrary_model_ids(self):
        self.assertTrue(hasattr(backend, 'model'), 'curated model lookup required')
        with self.assertRaises(ValueError):
            backend.model('../../etc')

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
