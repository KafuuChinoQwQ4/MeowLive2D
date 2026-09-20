"""Exercise cache pruning against real files and Cargo locks, without deleting source."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("rust_cache.py")
SPEC = importlib.util.spec_from_file_location("rust_cache", SCRIPT)
cache = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(cache)


class CacheTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="meowlive-rust-cache-")
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.target = self.root / "target"
        self.profile = self.target / "debug"
        (self.profile / ".fingerprint").mkdir(parents=True)
        (self.profile / ".cargo-lock").touch()
        self.source = self.root / "src/lib.rs"
        self.source.parent.mkdir()
        self.source.write_text("pub fn value() -> u32 { 1 }\n")

    def artifact(self, hash_, *, profile=1, feature="[]", test=True, library=False):
        name = "demo"
        kind = "lib-demo" if library else "test-integration-test-demo" if test else "bin-demo"
        fp = self.profile / ".fingerprint" / f"demo-{hash_}"
        fp.mkdir(parents=True, exist_ok=True)
        recipe = dict(rustc=1, features=feature, declared_features="[]", target=1,
                      profile=profile, path=1, rustflags=[], config=1, compile_kind=0,
                      deps=[], local=[])
        (fp / f"{kind}.json").write_text(json.dumps(recipe))
        filename = f"libdemo-{hash_}.rlib" if library else f"demo-{hash_}"
        output = self.profile / "deps" / filename
        output.parent.mkdir(exist_ok=True)
        output.write_bytes(b"compiled output")
        (output.parent / f"demo-{hash_}.d").write_text("dep info")
        return {"reason": "compiler-artifact", "package_id": "path+file:///fixture#demo@0.1.0",
                "manifest_path": str(self.root / "Cargo.toml"),
                "target": {"name": name, "kind": ["lib" if library else "test" if test else "bin"],
                           "src_path": str(self.source)}, "profile": {"test": test},
                "filenames": [str(output)], "executable": None if library else str(output),
                "fresh": True}

    def record(self, context, artifacts, records=()):
        return cache.record_build(self.target, context, artifacts, records)

    def prune(self, dry_run=False):
        return cache.prune(self.target, self.root, dry_run=dry_run)

    def test_bootstrap_removes_superseded_executable_but_keeps_current_and_other_modes(self):
        old = self.artifact("1111111111111111")
        current = self.artifact("2222222222222222")
        other = self.artifact("3333333333333333", profile=2)
        feature = self.artifact("4444444444444444", feature='["extra"]')
        library = self.artifact("5555555555555555", library=True)
        self.record("tests", [current])
        preview = self.prune(dry_run=True)
        self.assertGreater(preview["bytes"], 0)
        self.assertTrue(Path(old["executable"]).exists())
        self.prune()
        self.assertFalse(Path(old["executable"]).exists())
        for artifact in [current, other, feature, library]:
            self.assertTrue(Path(artifact["filenames"][0]).exists())
        self.assertTrue(self.source.exists())

    def test_fresh_output_is_protected_even_if_its_timestamp_is_old(self):
        current = self.artifact("1111111111111111")
        os.utime(current["executable"], (1, 1))
        obsolete = self.artifact("2222222222222222")
        self.record("tests", [current])
        self.prune()
        self.assertTrue(Path(current["executable"]).exists())
        self.assertFalse(Path(obsolete["executable"]).exists())

    def test_preserves_outputs_used_by_another_command_context(self):
        workspace = self.artifact("1111111111111111")
        package = self.artifact("2222222222222222")
        self.record("workspace", [workspace])
        self.record("package", [package])
        self.prune()
        self.assertTrue(Path(workspace["executable"]).exists())
        self.assertTrue(Path(package["executable"]).exists())
        newest = self.artifact("3333333333333333")
        self.record("workspace", [newest])
        self.prune()
        self.assertFalse(Path(workspace["executable"]).exists())
        self.assertTrue(Path(package["executable"]).exists())

    def test_only_retires_previously_recorded_incremental_directories(self):
        old = self.artifact("1111111111111111")
        current = self.artifact("2222222222222222")
        paths = [self.profile / "incremental" / f"demo-{letter * 13}" for letter in "abc"]
        for p in paths:
            p.mkdir(parents=True)
            (p / "dep-graph.bin").write_bytes(b"cached")
        self.record("tests", [old], [{"hash": "1111111111111111", "profile": str(self.profile),
                                     "incremental": [str(paths[0])]}])
        self.record("tests", [current], [{"hash": "2222222222222222", "profile": str(self.profile),
                                         "incremental": [str(paths[1])]}])
        self.prune()
        self.assertFalse(paths[0].exists())
        self.assertTrue(paths[1].exists())
        self.assertTrue(paths[2].exists(), "unknown pre-existing caches are never guessed obsolete")

    def test_skips_pruning_while_direct_cargo_holds_its_lock(self):
        if cache.fcntl is None:
            self.skipTest("Cargo flock test requires Unix")
        old = self.artifact("1111111111111111")
        current = self.artifact("2222222222222222")
        self.record("tests", [current])
        locker = subprocess.Popen(["python3", "-u", "-c",
            "import fcntl,sys; f=open(sys.argv[1],'r+'); fcntl.flock(f,fcntl.LOCK_EX); print('locked'); sys.stdin.read()",
            str(self.profile / ".cargo-lock")], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        self.addCleanup(lambda: locker.poll() is None and locker.kill())
        self.assertEqual(locker.stdout.readline().strip(), "locked")
        self.assertTrue(self.prune()["skipped"])
        self.assertTrue(Path(old["executable"]).exists())
        locker.communicate("")

    def test_never_follows_symlinks_or_deletes_cross_tools(self):
        old = self.artifact("1111111111111111")
        current = self.artifact("2222222222222222")
        self.record("tests", [current])
        p = Path(old["executable"])
        p.unlink()
        p.symlink_to(self.source)
        tool = self.target / "cross-tools" / "demo-1111111111111111"
        tool.parent.mkdir()
        tool.write_text("toolchain")
        self.prune()
        self.assertTrue(p.is_symlink())
        self.assertTrue(self.source.exists())
        self.assertEqual(tool.read_text(), "toolchain")

    def test_missing_or_damaged_manifest_fails_closed(self):
        old = self.artifact("1111111111111111")
        self.assertEqual(self.prune()["files"], 0)
        state = self.target / ".rust-cache" / "state.json"
        state.parent.mkdir(exist_ok=True)
        state.write_text("{broken")
        with self.assertRaises(ValueError):
            self.prune()
        self.assertTrue(Path(old["executable"]).exists())

    def test_real_cargo_keeps_warm_builds_and_records_incremental_replacements(self):
        # No registry/network dependencies. Exercise build -> check -> test -> cleanup -> all three again.
        (self.root / "Cargo.toml").write_text('[package]\nname="demo"\nversion="0.1.0"\nedition="2021"\n')
        scripts = self.root / "scripts"
        scripts.mkdir()
        copy = scripts / "rust_cache.py"
        copy.write_bytes(SCRIPT.read_bytes())
        copy.chmod(0o755)
        # Remove simulated files; this test uses real Cargo metadata and locks only.
        import shutil
        shutil.rmtree(self.target)
        env = {**os.environ, "PYTHONDONTWRITEBYTECODE": "1"}
        for key in ["RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "CARGO_TARGET_DIR", "CARGO_BUILD_BUILD_DIR"]:
            env.pop(key, None)
        def run(*args):
            return subprocess.run([sys.executable, str(copy), *args], cwd=self.root,
                                  capture_output=True, text=True, env=env, timeout=60)
        for command in ["build", "check", "test"]:
            result = run(command, "--offline")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertNotIn("failed to connect to jobserver", result.stderr)
        state = json.loads((self.target / ".rust-cache/state.json").read_text())
        self.assertTrue(state["compilers"], "real rustc must record incremental cache ownership")
        # A failed compilation must preserve the last successful inventory.
        before = (self.target / ".rust-cache/state.json").read_bytes()
        self.source.write_text("invalid rust syntax\n")
        self.assertNotEqual(run("build", "--offline").returncode, 0)
        self.assertEqual((self.target / ".rust-cache/state.json").read_bytes(), before)
        self.source.write_text("pub fn value() -> u32 { 2 }\n")
        for command in ["build", "check", "test"]:
            self.assertEqual(run(command, "--offline").returncode, 0)
        self.assertEqual(run("clean", "--bootstrap").returncode, 0)
        for command in ["build", "check", "test"]:
            result = run(command, "--offline")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("编译 0", result.stderr)
        example = self.root / "examples/hello.rs"
        example.parent.mkdir()
        example.write_text('fn main() { println!("{}", std::env::args().nth(1).unwrap()); }\n')
        result = run("run", "--example", "hello", "--offline", "--", "argument forwarded")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("argument forwarded", result.stdout)
        passthrough = self.root / "workspace-wrapper"
        passthrough.write_text('#!/bin/sh\nexec "$@"\n')
        passthrough.chmod(0o755)
        env["RUSTC_WORKSPACE_WRAPPER"] = str(passthrough)
        result = run("check", "--offline")
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_cargo_output_written_after_inventory_is_not_assumed_obsolete(self):
        import time
        current = self.artifact("1111111111111111")
        self.record("tests", [current])
        direct_cargo = self.artifact("2222222222222222")
        fp = self.profile / ".fingerprint/demo-2222222222222222/test-integration-test-demo.json"
        os.utime(fp, (time.time() + 10, time.time() + 10))
        self.prune()
        self.assertTrue(Path(direct_cargo["executable"]).exists())


if __name__ == "__main__":
    unittest.main()
