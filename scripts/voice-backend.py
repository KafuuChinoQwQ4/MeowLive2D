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
LEGACY_ENGINE = Path('/data/third_party/GPT-SoVITS')
LEGACY_PYTHON = Path('/data/micromamba/envs/gpt-sovits/bin/python')
CATALOG = json.loads(Path(__file__).with_name('voice-backend-models.json').read_text())
REVISION = '48b1a0169a28582a8984402f82cf438d3bfa6aca'


def model(identifier):
    for item in CATALOG:
        if item['id'] == identifier:
            return item
    raise ValueError('未知模型')


def downloaded(item, root):
    markers = list(item['markers'])
    if item['id'] == 'gpt-sovits-v2' or 'G2PWModel.zip' in markers:
        markers = [name for name in markers if name != 'G2PWModel.zip']
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


def installation():
    """Managed installs take precedence; registered existing installs stay read-only."""
    if (ROOT / 'backend.json').is_file():
        return ROOT / 'engine', ROOT / 'venv/bin/python', True
    registration = ROOT / 'existing.json'
    if registration.is_file():
        paths = json.loads(registration.read_text())
        return Path(paths['engine']), Path(paths['python']), False
    if LEGACY_ENGINE.is_dir() and LEGACY_PYTHON.is_file():
        return LEGACY_ENGINE, LEGACY_PYTHON, False
    return ROOT / 'engine', ROOT / 'venv/bin/python', True


def model_directory(item, engine, managed):
    if item['id'] == 'gpt-sovits-v2' and not managed and downloaded(item, engine):
        return engine
    return ROOT / 'models' / item['id']


def probe(existing=None):
    engine, python, managed = ROOT / 'engine', ROOT / 'venv/bin/python', True
    ready, gpu = False, False
    try:
        engine, python, managed = existing or installation()
        if not engine.is_absolute() or not python.is_absolute():
            raise ValueError('训练 Python 和引擎目录必须是 WSL 绝对路径')
        if managed:
            manifest = json.loads((ROOT / 'backend.json').read_text())
            if manifest != {'schema': 1, 'revision': REVISION}:
                raise ValueError('后端版本不兼容，请重新安装')
        for name in ['api_v2.py', 'GPT_SoVITS/s1_train.py', 'GPT_SoVITS/s2_train.py']:
            if not (engine / name).is_file():
                raise ValueError('引擎代码缺失：' + name)
        if managed:
            revision = subprocess.check_output(['git', '-C', str(engine), 'rev-parse', 'HEAD'], text=True, timeout=10).strip()
            if revision != REVISION:
                raise ValueError('引擎版本与受管后端不兼容')
        result = subprocess.run([str(python), '-s', '-c',
            'import json, torch, faster_whisper, transformers, fastapi, librosa, soundfile; print(json.dumps(torch.cuda.is_available()))'],
            capture_output=True, text=True, timeout=60)
        if result.returncode:
            raise ValueError(result.stderr.strip()[-500:] or '训练后端依赖不完整')
        gpu = json.loads(result.stdout.strip().splitlines()[-1]) is True
        ready = True
        detail = ('已发现现有 GPT-SoVITS 训练环境。' if not managed else '') + ('CUDA GPU 可用' if gpu else 'CPU 推理可用；训练需要 CUDA GPU')
    except Exception as error:
        detail = '后端检测未通过：' + str(error)
    return {'backend': {'ready': ready, 'engineRoot': str(engine),
                        'pythonPath': str(python), 'modelRoot': str(model_directory(model('gpt-sovits-v2'), engine, managed)), 'gpu': gpu, 'detail': detail},
            'models': [{'id': item['id'], 'name': item['name'],
                        'capability': ('transcription' if item.get('purpose') == 'asr' else 'training_inference')
                        if item['compatibility'] == 'ready' else 'download_only',
                        'downloaded': downloaded(item, model_directory(item, engine, managed)), 'selected': False}
                       for item in CATALOG], 'inferenceRunning': running()}


def download(identifier):
    from huggingface_hub import snapshot_download
    item = model(identifier)
    destination = ROOT / 'models' / identifier
    for index, source in enumerate(item['sources']):
        print(f"下载 {source['repo']} ({index + 1}/{len(item['sources'])})", flush=True)
        snapshot_download(repo_id=source['repo'], local_dir=str(destination / source['prefix']),
                          allow_patterns=[value + '*' if value.endswith('/') else value for value in source['include']])
    if 'G2PWModel.zip' in item['markers']:
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
    engine, python, managed = installation()
    models = model_directory(item, engine, managed)
    if not downloaded(item, models):
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
        child = subprocess.Popen([str(python), '-s', str(ROOT / 'scripts/start-managed-inference.py'),
            '--engine-root', str(engine), '--data-dir', str(runtime / 'inference'),
            '--model-root', str(models), '--device', 'cuda' if state['backend']['gpu'] else 'cpu'],
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
    parser.add_argument('--existing-python')
    parser.add_argument('--existing-engine')
    args = parser.parse_args()
    if args.action in {'probe', 'download', 'start'}:
        if os.getpgrp() != os.getpid():
            os.setsid()
        record_process(ROOT / 'runtime/operation.pid', os.getpid())
    if args.existing_python or args.existing_engine:
        if args.action != 'probe' or not (args.existing_python and args.existing_engine):
            parser.error('已有环境需要同时指定 Python 和引擎目录，仅用于检测')
        if (ROOT / 'backend.json').is_file():
            raise ValueError('已安装专用后端，无需导入已有环境')
        current_engine, current_python, _ = installation()
        if running() and (Path(args.existing_engine), Path(args.existing_python)) != (current_engine, current_python):
            raise ValueError('请先停止语音引擎，再切换已有训练环境')
        state = probe((Path(args.existing_engine), Path(args.existing_python), False))
        if state['backend']['ready']:
            ROOT.mkdir(parents=True, exist_ok=True)
            pending = ROOT / 'existing.json.tmp'
            pending.write_text(json.dumps({'engine': args.existing_engine, 'python': args.existing_python}))
            pending.replace(ROOT / 'existing.json')
        print(json.dumps(state, ensure_ascii=False), flush=True)
        return
    if args.action == 'status':
        print(json.dumps({'inferenceRunning': running()}))
        return
    if args.action == 'cancel':
        stop(ROOT / 'runtime/operation.pid')
        return
    if args.action == 'stop':
        stop(ROOT / 'runtime/inference.pid')
        return
    if args.action == 'download':
        _, python, _ = installation()
        if Path(sys.executable).absolute() != python.absolute():
            os.execv(str(python), [str(python), '-s', __file__, 'download', args.model])
        download(args.model)
    elif args.action == 'start':
        start()
    print(json.dumps(probe(), ensure_ascii=False), flush=True)

if __name__ == '__main__':
    main()
