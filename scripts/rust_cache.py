#!/usr/bin/env python3
"""Run Cargo with a successful-build inventory; retire superseded outputs, never age out live caches."""
import contextlib
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import time

try:
    import fcntl
except ImportError:
    fcntl = None  # Builds work on Windows; automatic deletion requires Cargo-compatible Unix locks.

HASH = re.compile(r"-([0-9a-f]{16})(?:\.[^.]+)?$")
INCREMENTAL = re.compile(r"[\w]+-[0-9a-z]{13}$")
ROOT = Path(__file__).resolve().parent.parent
STATE_DIR = ".rust-cache"


def safe_path(target, relative):
    """Only ordinary paths within this target; symlink ancestors must never be traversed."""
    p = Path(relative)
    if p.is_absolute() or ".." in p.parts or not p.parts:
        raise ValueError("invalid cache path")
    result = target
    for part in p.parts:
        result = result / part
        if result.is_symlink():
            raise ValueError("symlink in cache path")
    return result


def state_path(target):
    if target.is_symlink() or not target.is_dir():
        raise ValueError("target must be an ordinary project directory")
    return safe_path(target, f"{STATE_DIR}/state.json")


def load_state(target):
    p = state_path(target)
    if not p.exists():
        return {"schema": 1, "contexts": {}, "retired": [], "compilers": {}}
    state = json.loads(p.read_text())
    if state.get("schema") != 1 or not isinstance(state.get("contexts"), dict):
        raise ValueError("unrecognized Rust cache inventory; refusing cleanup")
    return state


def save_state(target, state):
    p = state_path(target)
    p.parent.mkdir(exist_ok=True)
    with tempfile.NamedTemporaryFile(mode="w", dir=p.parent, delete=False) as out:
        json.dump(state, out, ensure_ascii=False)
        temp = out.name
    os.replace(temp, p)


def profile_of(target, file):
    relative = Path(file).relative_to(target)
    parts = relative.parts
    index = parts.index("deps") if "deps" in parts else len(parts) - 1
    profile = Path(*parts[:index])
    # Leave cross-tools, nested probe projects, custom build-dir layouts and release bundles alone.
    if len(profile.parts) not in (1, 2) or profile.name not in ("debug", "release"):
        return None
    safe_path(target, profile)
    return profile.as_posix()


def fingerprint_entries(target, profile):
    directory = safe_path(target, f"{profile}/.fingerprint")
    if not directory.is_dir():
        return
    for folder in sorted(directory.iterdir()):
        if folder.is_symlink() or not folder.is_dir():
            continue
        match = HASH.search(folder.name)
        if not match:
            continue
        for info in folder.glob("*.json"):
            if info.is_symlink():
                continue
            try:
                data = json.loads(info.read_text())
                # Source/dependency revision changes produce replacements, not new configurations.
                recipe = {k: v for k, v in data.items() if k not in ("local", "deps")}
                recipe["kind"] = info.name
                recipe["package"] = folder.name[:-17]
                recipe["directory"] = profile
                yield match[1], json.dumps(recipe, sort_keys=True), info.name, folder
            except (OSError, ValueError):
                continue


def inventory(target, artifacts):
    profiles, rows = {}, []
    for artifact in artifacts:
        filenames = artifact.get("filenames", [])
        if not filenames or "custom-build" in artifact["target"]["kind"]:
            continue
        try:
            profile = profile_of(target, filenames[0])
        except ValueError:
            continue
        if not profile:
            continue
        if profile not in profiles:
            profiles[profile] = list(fingerprint_entries(target, profile))
        hashes = {m[1] for f in filenames if (m := HASH.search(Path(f).name))}
        # Cargo reports final binaries without a hash. Their deps hardlinks identify the actual unit.
        executable = artifact.get("executable")
        if executable and not hashes:
            name = artifact["target"]["name"].replace("-", "_")
            for candidate in safe_path(target, f"{profile}/deps").glob(f"{name}-*"):
                if candidate.is_symlink() or candidate.suffix:
                    continue
                if candidate.exists() and Path(executable).exists() and candidate.samefile(executable):
                    if match := HASH.search(candidate.name):
                        hashes.add(match[1])
        for hash_ in hashes:
            for found_hash, recipe, kind, folder in profiles[profile]:
                if hash_ != found_hash:
                    continue
                rows.append({"profile": profile, "hash": hash_, "recipe": recipe,
                             "executable": bool(executable),
                             "fingerprint": folder.relative_to(target).as_posix()})
    return rows


def record_build(target, context, artifacts, compiler_records=(), *, started_at=None):
    state = load_state(target)
    entries = inventory(target, artifacts)
    old = state["contexts"].get(context, {}).get("entries", [])
    state["retired"].extend(old)
    state["contexts"][context] = {"entries": entries, "used_at": started_at or time.time()}
    for record in compiler_records:
        try:
            profile = Path(record["profile"]).relative_to(target).as_posix()
            paths = [Path(p).relative_to(target).as_posix() for p in record["incremental"]]
            for p in paths:
                safe_path(target, p)
            state["compilers"][f'{profile}:{record["hash"]}'] = paths
        except (KeyError, ValueError):
            continue
    save_state(target, state)
    return entries


def unit(entry):
    return f'{entry["profile"]}:{entry["hash"]}'


def size_and_files(p, inodes):
    if p.is_symlink():
        return 0, 0
    s = p.stat()
    inode = (s.st_dev, s.st_ino)
    size = 0 if inode in inodes else getattr(s, "st_blocks", 0) * 512
    inodes.add(inode)
    if not p.is_dir():
        return size, 1
    files = 0
    for child in p.iterdir():
        child_size, child_files = size_and_files(child, inodes)
        size += child_size
        files += child_files
    return size, files


@contextlib.contextmanager
def cargo_locks(target, profiles):
    if fcntl is None:
        yield False
        return
    with contextlib.ExitStack() as stack:
        try:
            for profile in sorted(profiles):
                lock_path = safe_path(target, f"{profile}/.cargo-lock")
                if not lock_path.exists():
                    yield False
                    return
                lock = stack.enter_context(lock_path.open("r+"))
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            yield False
            return
        yield True


def prune(target, root, *, dry_run=False, bootstrap=True):
    if target.resolve() != (root.resolve() / "target"):
        raise ValueError("cleanup is restricted to this project's target directory")
    state = load_state(target)
    live = [e for c in state["contexts"].values() for e in c["entries"]]
    protected = {unit(e) for e in live}
    profiles = {e["profile"] for e in live}
    observed_at = max((c["used_at"] for c in state["contexts"].values()), default=0)
    result = {"bytes": 0, "files": 0, "skipped": False}
    if not live:
        return result
    with cargo_locks(target, profiles) as locked:
        if not locked:
            result["skipped"] = True
            return result
        # Only executable generations are bootstrapped. Library/build-script dependencies stay intact.
        candidates = [e for e in state["retired"] if e.get("executable")]
        if bootstrap:
            recipes = {e["recipe"] for e in live if e.get("executable")}
            for profile in profiles:
                for hash_, recipe, _, folder in fingerprint_entries(target, profile):
                    if recipe in recipes:
                        candidates.append({"profile": profile, "hash": hash_, "recipe": recipe,
                                           "fingerprint": folder.relative_to(target).as_posix()})
        paths = set()
        deps_by_profile = {}
        for profile in profiles:
            indexed = {}
            for p in safe_path(target, f"{profile}/deps").iterdir():
                if match := HASH.search(p.name):
                    indexed.setdefault(match[1], []).append(p)
            deps_by_profile[profile] = indexed
        for entry in candidates:
            if unit(entry) in protected:
                continue
            folder = safe_path(target, entry["fingerprint"])
            # A bare Cargo command may have completed between our build and acquiring the lock.
            # Its newly written fingerprint takes precedence over an older inventory.
            if folder.is_dir() and any(p.stat().st_mtime > observed_at for p in folder.glob("*.json")):
                continue
            for p in deps_by_profile.get(entry["profile"], {}).get(entry["hash"], []):
                if not p.suffix or p.suffix in (".d", ".exe", ".pdb"):
                    paths.add(p)
            paths.add(target / entry["fingerprint"])
        current_incremental = {p for k, ps in state["compilers"].items() if k in protected for p in ps}
        retired_compilers = []
        for key, directories in state["compilers"].items():
            if key in protected:
                continue
            retired_compilers.append(key)
            for relative in directories:
                p = Path(relative)
                if relative not in current_incremental and p.parent.name == "incremental" and INCREMENTAL.fullmatch(p.name):
                    directory = safe_path(target, relative)
                    if directory.exists() and directory.stat().st_mtime <= observed_at:
                        paths.add(directory)
        inodes = set()
        for p in sorted(paths):
            try:
                p = safe_path(target, p.relative_to(target))
            except ValueError:
                continue
            if not p.exists():
                continue
            size, files = size_and_files(p, inodes)
            result["bytes"] += size
            result["files"] += files
            if not dry_run:
                shutil.rmtree(p) if p.is_dir() else p.unlink()
        if not dry_run:
            state["retired"] = []
            for key in retired_compilers:
                del state["compilers"][key]
            save_state(target, state)
    return result


def compiler_mode():
    """Record incremental directories actually touched by rustc; don't infer identity from age."""
    args = sys.argv[2:]
    def value(flag):
        return args[args.index(flag) + 1] if flag in args else None
    name, output = value("--crate-name"), value("--out-dir")
    codegen = [args[i + 1] for i, arg in enumerate(args[:-1]) if arg == "-C"]
    inc = next((a.split("=", 1)[1] for a in codegen if a.startswith("incremental=")), None)
    hash_ = next((a.removeprefix("extra-filename=-") for a in codegen if a.startswith("extra-filename=-")), None)
    def snapshot():
        if not inc or not name:
            return {}
        return {str(p): p.stat().st_mtime_ns for p in Path(inc).glob(f"{name}-*") if p.is_dir() and not p.is_symlink()}
    before = snapshot()
    command = [sys.argv[1], *args]
    if wrapper := os.environ.get("MEOWLIVE_PREVIOUS_RUSTC_WRAPPER"):
        command.insert(0, wrapper)
    # Cargo's inherited jobserver descriptors must reach rustc and native build scripts.
    status = subprocess.call(command, close_fds=False)
    if status == 0 and hash_ and output:
        after = snapshot()
        touched = [p for p, stamp in after.items() if before.get(p) != stamp]
        if touched:
            record = {"hash": hash_, "profile": str(Path(output).parent), "incremental": touched}
            with tempfile.NamedTemporaryFile(mode="w", suffix=".json", dir=os.environ["MEOWLIVE_RUST_CACHE_JOURNAL"], delete=False) as out:
                json.dump(record, out)
    return status


def context_key(args, env):
    # Test execution/filtering doesn't change its compiled outputs; no-run inventories protect normal tests.
    build_args = args[:args.index("--")] if "--" in args else args
    build_args = [a for a in build_args if a not in ("--no-run", "--locked", "--offline", "--frozen", "-v", "-vv")]
    flags = {k: env.get(k) for k in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_INCREMENTAL", "RUSTUP_TOOLCHAIN")}
    flags.update({k: v for k, v in env.items() if k in ("RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER")
                  or k.startswith(("CARGO_PROFILE_", "CARGO_BUILD_", "CARGO_TARGET_"))})
    return hashlib.sha256(json.dumps([build_args, flags], sort_keys=True).encode()).hexdigest()


def report(result, dry_run=False):
    if result["skipped"]:
        print("Rust 缓存：其他 Cargo 正在构建或不支持文件锁，本次跳过清理。", file=sys.stderr)
    else:
        print(f'Rust 缓存：{"预计回收" if dry_run else "已回收"} {result["bytes"] / 1024**3:.2f} GiB / {result["files"]} 个文件；保留当前产物与增量缓存。', file=sys.stderr)


def main(args):
    if not args or args[0] in ("--help", "-h"):
        print("用法：python3 scripts/rust_cache.py <build|check|test|run> [Cargo 参数]\n"
              "      python3 scripts/rust_cache.py clean [--dry-run] [--bootstrap]\n"
              "构建成功后回收有记录的旧产物；bootstrap 只清理与当前配置相同的旧可执行程序。")
        return 0
    if args[0] not in ("build", "check", "test", "run", "clean"):
        raise ValueError("unsupported Cargo command")
    os.chdir(ROOT)
    if os.environ.get("CARGO_TARGET_DIR") or os.environ.get("CARGO_BUILD_BUILD_DIR"):
        raise ValueError("自定义 Cargo 输出目录请直接调用 cargo；清理仅支持本项目 target")
    target = ROOT / "target"
    target.mkdir(exist_ok=True)
    state_path(target).parent.mkdir(exist_ok=True)
    lock_path = safe_path(target, f"{STATE_DIR}/runner.lock")
    with lock_path.open("a+") as lock:
        if fcntl:
            fcntl.flock(lock, fcntl.LOCK_EX)
        if args[0] == "clean":
            if set(args[1:]) - {"--dry-run", "--bootstrap"}:
                raise ValueError("unknown cleanup option")
            report(prune(target, ROOT, dry_run="--dry-run" in args, bootstrap="--bootstrap" in args), "--dry-run" in args)
            return 0
        run_command = None
        if args[0] == "run":
            run_command = ["cargo", *args]
            split = args.index("--") if "--" in args else len(args)
            args = ["build", *args[1:split]]
        if any(a.startswith(("--message-format", "--target-dir", "--manifest-path", "--config")) for a in args):
            raise ValueError("custom output/config arguments must use cargo directly")
        env = dict(os.environ)
        context = context_key(args, env)
        artifacts, fresh, compiled = [], 0, 0
        started = time.monotonic()
        started_at = time.time()
        with tempfile.TemporaryDirectory(prefix="compile-", dir=state_path(target).parent) as journal:
            env["MEOWLIVE_RUST_CACHE_JOURNAL"] = journal
            if env.get("RUSTC_WRAPPER"):
                env["MEOWLIVE_PREVIOUS_RUSTC_WRAPPER"] = env["RUSTC_WRAPPER"]
            env["RUSTC_WRAPPER"] = str(Path(__file__).resolve())
            split = args.index("--") if "--" in args else len(args)
            command = ["cargo", *args[:split], "--message-format=json-render-diagnostics", *args[split:]]
            with subprocess.Popen(command, stdout=subprocess.PIPE, text=True, env=env) as child:
                for line in child.stdout:
                    try:
                        message = json.loads(line)
                    except ValueError:
                        print(line, end="", flush=True)
                        continue
                    if not isinstance(message, dict) or "reason" not in message:
                        print(line, end="", flush=True)
                        continue
                    if message["reason"] == "compiler-artifact":
                        fresh += bool(message.get("fresh"))
                        compiled += not message.get("fresh", False)
                        manifest = Path(message.get("manifest_path", "/"))
                        if manifest.is_relative_to(ROOT):
                            artifacts.append(message)
                    elif message["reason"] == "compiler-message":
                        print(message["message"].get("rendered", ""), end="", file=sys.stderr, flush=True)
                code = child.wait()
            print(f"Rust 构建：缓存命中 {fresh}，编译 {compiled}，耗时 {time.monotonic() - started:.2f}s。", file=sys.stderr)
            if code:
                return code  # Failed builds and tests never replace the inventory or prune anything.
            records = [json.loads(p.read_text()) for p in Path(journal).glob("*.json")]
            record_build(target, context, artifacts, records, started_at=started_at)
            report(prune(target, ROOT, bootstrap=False))
        if run_command is None:
            return 0
    # Let Cargo preserve target runners, dynamic-library search paths and executable selection.
    # It rechecks the already-built graph without compiling, and holds no wrapper lock while running.
    return subprocess.call(run_command)


if __name__ == "__main__":
    try:
        is_compiler = os.environ.get("MEOWLIVE_RUST_CACHE_JOURNAL") and len(sys.argv) > 1 and sys.argv[1] not in (
            "build", "check", "test", "run", "clean", "--help", "-h",
        )
        sys.exit(compiler_mode() if is_compiler else main(sys.argv[1:]))
    except (OSError, ValueError) as error:
        print(f"Rust 缓存：{error}", file=sys.stderr)
        sys.exit(1)
