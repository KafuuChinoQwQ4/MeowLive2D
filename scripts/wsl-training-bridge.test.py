import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).with_name('wsl-training-bridge.py')

class BridgeTests(unittest.TestCase):
    def test_windows_path_is_single_wslpath_argument(self):
        spec = importlib.util.spec_from_file_location('bridge', SCRIPT)
        bridge = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(bridge)
        with patch.object(bridge.subprocess, 'check_output', return_value='/mnt/c/a b/audio.wav\n') as run:
            self.assertEqual(bridge.linux_path(r'C:\a b\audio.wav'), '/mnt/c/a b/audio.wav')
            self.assertEqual(run.call_args.args[0], ['wslpath', '-a', '-u', r'C:\a b\audio.wav'])
        self.assertEqual(bridge.linux_path('/opt/models/a'), '/opt/models/a')

    def test_wsl_unc_paths_target_only_current_distribution(self):
        spec = importlib.util.spec_from_file_location('bridge', SCRIPT)
        bridge = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(bridge)
        with patch.dict(bridge.os.environ, {'WSL_DISTRO_NAME': 'Ubuntu'}):
            self.assertEqual(bridge.linux_path(r'\\?\UNC\wsl.localhost\Ubuntu\home\voice data\a.wav'), '/home/voice data/a.wav')
            self.assertEqual(bridge.linux_path(r'\\wsl$\Ubuntu\opt\model.ckpt'), '/opt/model.ckpt')
            with self.assertRaises(ValueError):
                bridge.linux_path(r'\\wsl.localhost\Other\opt\model.ckpt')

    def test_transcription_arguments_and_child_stdin_are_isolated(self):
        import json
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            runner = root / 'transcribe-training.py'
            runner.write_text('import sys,json,os\nprint(json.dumps({"args":sys.argv[1:],"stdin":sys.stdin.read(),"model":os.environ["MEOWLIVE_ASR_MODEL"]}),flush=True)\n')
            with subprocess.Popen([sys.executable, str(SCRIPT), '--runner', str(runner), '--engine-root', folder, '--audio', str(root/'audio.wav'), '--language', 'zh', '--model', '/opt/asr model'], stdin=subprocess.PIPE, stdout=subprocess.PIPE) as child:
                value = json.loads(child.stdout.readline())
                child.wait(timeout=5)
                self.assertEqual(child.returncode, 0)
                self.assertEqual(value, {'args': ['--audio', str(root/'audio.wav'), '--language', 'zh', '--engine-root', folder, '--model', '/opt/asr model'], 'stdin': '', 'model': '/opt/asr model'})

    def test_stdin_disconnect_terminates_descendants(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            runner = root / 'runner.py'
            marker = root / 'late-write'
            runner.write_text('import subprocess,sys,time\nsubprocess.Popen([sys.executable,"-c",'+repr('import time,pathlib,signal; signal.signal(signal.SIGTERM, signal.SIG_IGN); time.sleep(2); pathlib.Path('+repr(str(marker))+').touch()')+'])\nprint("ready",flush=True)\ntime.sleep(20)\n')
            child = subprocess.Popen([sys.executable, str(SCRIPT), '--runner', str(runner), '--engine-root', folder, '--job', str(root/'job.json')], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
            self.assertEqual(child.stdout.readline().strip(), b'ready')
            child.stdin.close()
            child.wait(timeout=5)
            child.stdout.close()
            time.sleep(2.2)
            self.assertFalse(marker.exists())

if __name__ == '__main__': unittest.main()
