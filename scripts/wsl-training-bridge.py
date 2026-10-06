#!/usr/bin/env python3
"""Run a Windows-owned training job in WSL; stdin lifetime owns the Linux process group."""
import argparse
import importlib.util
import os
from pathlib import Path
import signal
import subprocess
import sys
import threading
import time


def linux_path(value):
    if value.startswith('/'):
        return value
    if value.startswith('\\\\?\\'):
        value = value[4:]
        if value.startswith('UNC\\'):
            value = '\\\\' + value[4:]
    parts = value.split('\\')
    if len(parts) >= 4 and parts[:2] == ['', ''] and parts[2].lower() in {'wsl.localhost', 'wsl$'}:
        if parts[3].casefold() != os.environ.get('WSL_DISTRO_NAME', '').casefold():
            raise ValueError('WSL resource belongs to another distribution')
        return '/' + '/'.join(parts[4:])
    result = subprocess.check_output(['wslpath', '-a', '-u', value], text=True, timeout=10).strip()
    if not result.startswith('/'):
        raise ValueError('WSL path conversion failed')
    return result


def terminate_group(child):
    for sig in (signal.SIGTERM, signal.SIGKILL):
        try:
            os.killpg(child.pid, sig)
        except ProcessLookupError:
            break
        if sig == signal.SIGTERM:
            time.sleep(0.2)
    child.wait()
    # SIGKILL is asynchronous. Keep the host task/GPU slot until every live
    # member has gone; zombies have already released device resources.
    while group_running(child.pid):
        time.sleep(0.01)


def group_running(group):
    for process in Path('/proc').iterdir():
        if not process.name.isdigit():
            continue
        try:
            fields = (process / 'stat').read_text().rsplit(') ', 1)[1].split()
            if int(fields[2]) == group and fields[0] not in {'Z', 'X'}:
                return True
        except (OSError, ValueError, IndexError):
            continue
    return False


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--runner', required=True)
    parser.add_argument('--engine-root', required=True)
    parser.add_argument('--model')
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--job')
    mode.add_argument('--audio')
    parser.add_argument('--language')
    args = parser.parse_args()
    stop = threading.Event()
    def lease():
        while os.read(sys.stdin.fileno(), 1):
            pass
        stop.set()
    threading.Thread(target=lease, daemon=True).start()
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
        signal.signal(sig, lambda *_: stop.set())
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE='1', PYTHONUNBUFFERED='1', MEOWLIVE_GPT_SOVITS_ROOT=linux_path(args.engine_root))
    spec = importlib.util.spec_from_file_location('voice_backend', Path(__file__).with_name('voice-backend.py'))
    backend = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(backend)
    engine = Path(env['MEOWLIVE_GPT_SOVITS_ROOT'])
    models = backend.model_directory(backend.model('gpt-sovits-v2'), engine, engine == backend.ROOT / 'engine')
    env.setdefault('MEOWLIVE_GPT_SOVITS_MODELS', str(models))
    if args.model:
        env['MEOWLIVE_ASR_MODEL'] = linux_path(args.model)
    command = [sys.executable, '-s', args.runner]
    if args.job:
        path = linux_path(args.job)
        command += ['--job', path]
    else:
        path = linux_path(args.audio)
        command += ['--audio', path, '--language', args.language, '--engine-root', env['MEOWLIVE_GPT_SOVITS_ROOT']]
        if args.model:
            command += ['--model', env['MEOWLIVE_ASR_MODEL']]
    if stop.is_set():
        return 130
    child = subprocess.Popen(command, cwd=str(Path(path).parent), env=env, stdin=subprocess.DEVNULL, start_new_session=True)
    try:
        while child.poll() is None and not stop.wait(0.05):
            pass
        return 130 if stop.is_set() else child.returncode
    finally:
        terminate_group(child)


if __name__ == '__main__':
    sys.exit(main())
