#!/usr/bin/env python3
"""Installed WSL backend controller. All state stays in the application prefix."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import sys
import time
import urllib.request

ROOT = Path('/opt/meowlive-voice')
CATALOG = json.loads(Path(__file__).with_name('voice-backend-models.json').read_text())
REVISION = '48b1a0169a28582a8984402f82cf438d3bfa6aca'


def model(identifier):
    for item in CATALOG:
        if item['id'] == identifier:
            return item
    raise ValueError('未知模型')


def downloaded(item, root):
    markers = list(item['markers'])
    if item['id'] == 'gpt-sovits-v2':
        markers += ['GPT_SoVITS/text/G2PWModel/g2pW.onnx', 'GPT_SoVITS/text/G2PWModel/config.py']
    return all((root / name).is_file() and (root / name).stat().st_size > 0 for name in markers)


def identity(pid):
    # comm may contain spaces and parentheses: fields after its final ')' start at field 3.
    return Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()[19]


def same_process(record):
    try:
        pid = int(record['pid'])
        fields = Path(f'/proc/{pid}/stat').read_text().rsplit(')', 1)[1].split()
        return (fields[0] != 'Z' and fields[19] == record['start']
                and record.get('boot') == Path('/proc/sys/kernel/random/boot_id').read_text().strip())
    except (OSError, ValueError, KeyError, IndexError):
        return False


def record_process(path, pid):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({'pid': pid, 'start': identity(pid),
                                'boot': Path('/proc/sys/kernel/random/boot_id').read_text().strip()}))


def group_alive(group):
    for entry in Path('/proc').iterdir():
        if not entry.name.isdigit():
            continue
        try:
            fields = (entry / 'stat').read_text().rsplit(')', 1)[1].split()
            if fields[0] != 'Z' and int(fields[2]) == group:
                return True
        except (OSError, ValueError, IndexError):
            continue
    return False


def stop(path):
    if not path.exists():
        return
    record = json.loads(path.read_text())
    if same_process(record):
        if os.getpgid(record['pid']) != record['pid']:
            raise ValueError('受管进程组标识不匹配，拒绝停止')
        os.killpg(record['pid'], signal.SIGTERM)
        for _ in range(50):
            if not group_alive(record['pid']):
                break
            time.sleep(.1)
        if group_alive(record['pid']):
            os.killpg(record['pid'], signal.SIGKILL)
            for _ in range(30):
                if not group_alive(record['pid']):
                    break
                time.sleep(.1)
            else:
                raise RuntimeError('受管进程组未能停止，拒绝确认退出')
    path.unlink(missing_ok=True)


def running():
    path = ROOT / 'runtime/inference.pid'
    return path.exists() and same_process(json.loads(path.read_text()))


def probe():
    ready, gpu, detail = False, False, '尚未安装兼容后端'
    try:
        manifest = json.loads((ROOT / 'backend.json').read_text())
        if manifest != {'schema': 1, 'revision': REVISION}:
            raise ValueError('后端版本不兼容，请重新安装')
        import torch
        import faster_whisper
        import transformers
        import fastapi
        if not (ROOT / 'engine/api_v2.py').is_file():
            raise ValueError('引擎代码缺失')
        revision = subprocess.check_output(['git', '-C', str(ROOT / 'engine'), 'rev-parse', 'HEAD'], text=True, timeout=10).strip()
        if revision != REVISION:
            raise ValueError('引擎版本与受管后端不兼容')
        ready, gpu = True, torch.cuda.is_available()
        detail = 'CUDA GPU 可用' if gpu else 'CPU 推理可用；训练需要 CUDA GPU'
    except Exception as error:
        detail = str(error)
    return {'backend': {'ready': ready, 'engineRoot': str(ROOT / 'engine'),
                        'pythonPath': str(ROOT / 'venv/bin/python'), 'gpu': gpu, 'detail': detail},
            'models': [{'id': item['id'], 'name': item['name'],
                        'capability': ('transcription' if item.get('purpose') == 'asr' else 'training_inference')
                        if item['compatibility'] == 'ready' else 'download_only',
                        'downloaded': downloaded(item, ROOT / 'models' / item['id']), 'selected': False}
                       for item in CATALOG], 'inferenceRunning': running()}


def download(identifier):
    from huggingface_hub import snapshot_download
    item = model(identifier)
    destination = ROOT / 'models' / identifier
    for index, source in enumerate(item['sources']):
        print(f"下载 {source['repo']} ({index + 1}/{len(item['sources'])})", flush=True)
        snapshot_download(repo_id=source['repo'], local_dir=str(destination / source['prefix']),
                          allow_patterns=[value + '*' if value.endswith('/') else value for value in source['include']])
    if identifier == 'gpt-sovits-v2':
        spec = importlib.util.spec_from_file_location('archive', Path(__file__).with_name('extract-model-archive.py'))
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        module.extract(destination / 'G2PWModel.zip', destination / 'GPT_SoVITS/text')
    if not downloaded(item, destination):
        raise ValueError('模型文件不完整，请重试下载')


def lifecycle(enabled=None):
    request = urllib.request.Request('http://127.0.0.1:9880/meowlive/models')
    if enabled is not None:
        request = urllib.request.Request(request.full_url, data=json.dumps({'enabled': enabled}).encode(),
                                         headers={'Content-Type': 'application/json'})
    with urllib.request.urlopen(request, timeout=120 if enabled else 2) as response:
        return json.load(response)


def ensure_loaded():
    status = lifecycle()
    if status.get('state') in {'unloaded', 'failed'}:
        status = lifecycle(True)
    for _ in range(120):
        if status.get('state') == 'loaded':
            return
        if status.get('state') == 'failed':
            raise RuntimeError('受管推理模型加载失败')
        time.sleep(1)
        status = lifecycle()
    raise RuntimeError('受管推理模型加载超时')


def start():
    if running():
        ensure_loaded()
        return
    item = model('gpt-sovits-v2')
    if not downloaded(item, ROOT / 'models' / item['id']):
        raise ValueError('请先完整下载并选择 GPT-SoVITS v2')
    state = probe()
    if not state['backend']['ready']:
        raise ValueError(state['backend']['detail'])
    with socket.socket() as port:
        try:
            port.bind(('127.0.0.1', 9880))
        except OSError as error:
            raise RuntimeError('推理端口 9880 已被其他服务占用') from error
    runtime = ROOT / 'runtime'
    runtime.mkdir(parents=True, exist_ok=True)
    with (runtime / 'inference.log').open('ab') as log:
        child = subprocess.Popen([str(ROOT / 'venv/bin/python'), '-s', str(ROOT / 'scripts/start-managed-inference.py'),
            '--engine-root', str(ROOT / 'engine'), '--data-dir', str(runtime / 'inference'),
            '--model-root', str(ROOT / 'models/gpt-sovits-v2'), '--device', 'cuda' if state['backend']['gpu'] else 'cpu'],
            start_new_session=True, stdin=subprocess.DEVNULL, stdout=log, stderr=log,
            env={**os.environ, 'MEOWLIVE_AUTO_ENABLE_MODELS': '1'})
    record_process(runtime / 'inference.pid', child.pid)
    def cancelled(*_):
        raise RuntimeError('推理启动已取消')
    signal.signal(signal.SIGTERM, cancelled)
    try:
        for _ in range(180):
            if child.poll() is not None:
                raise RuntimeError('推理进程退出：' + (runtime / 'inference.log').read_text(errors='replace')[-4000:])
            try:
                with urllib.request.urlopen('http://127.0.0.1:9880/meowlive/models', timeout=1) as response:
                    status = json.load(response)
                    if status.get('state') == 'loaded':
                        return
                    if status.get('state') == 'failed':
                        raise RuntimeError('模型加载失败，请检查推理日志')
            except (OSError, ValueError):
                pass
            time.sleep(1)
        raise RuntimeError('推理启动超时')
    except BaseException:
        stop(runtime / 'inference.pid')
        raise


def main():
    os.umask(0o022)
    parser = argparse.ArgumentParser()
    parser.add_argument('action', choices=['probe', 'download', 'start', 'stop', 'cancel', 'status'])
    parser.add_argument('model', nargs='?')
    args = parser.parse_args()
    if args.action == 'status':
        print(json.dumps({'inferenceRunning': running()}))
        return
    if args.action == 'cancel':
        stop(ROOT / 'runtime/operation.pid')
        return
    if args.action == 'stop':
        stop(ROOT / 'runtime/inference.pid')
        return
    if args.action in {'download', 'start'}:
        if os.getpgrp() != os.getpid():
            os.setsid()
        record_process(ROOT / 'runtime/operation.pid', os.getpid())
    if args.action == 'download':
        download(args.model)
    elif args.action == 'start':
        start()
    print(json.dumps(probe(), ensure_ascii=False), flush=True)

if __name__ == '__main__':
    main()
