#!/usr/bin/env python3
"""End-to-end check of the built Linux daemon over a private D-Bus session.

Run inside `dbus-run-session` after `cargo build --locked --bins`. The daemon gets
throwaway XDG directories, so no real state, model, microphone or desktop is used.
"""
import json
import os
import pathlib
import signal
import stat
import subprocess
import tempfile
import time
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
TARGET = ROOT / "target" / "debug"
SERVICE = "com.yap.Yap"
PATH = "/com/yap/Yap/Dictation"
INTERFACE = "com.yap.Yap.Dictation1"


class DaemonJourney(unittest.TestCase):
    def setUp(self):
        if "DBUS_SESSION_BUS_ADDRESS" not in os.environ:
            self.skipTest("run under dbus-run-session")
        self.tmp = tempfile.TemporaryDirectory()
        base = pathlib.Path(self.tmp.name)
        runtime = base / "runtime"
        runtime.mkdir(mode=0o700)
        self.state_file = base / "data" / "yap" / "state.json"
        self.env = {
            **os.environ,
            "HOME": str(base / "home"),
            "XDG_DATA_HOME": str(base / "data"),
            "XDG_CONFIG_HOME": str(base / "config"),
            "XDG_CACHE_HOME": str(base / "cache"),
            "XDG_RUNTIME_DIR": str(runtime),
        }
        self.daemon = None
        self.start_daemon()

    def tearDown(self):
        self.stop_daemon()
        self.tmp.cleanup()

    def start_daemon(self):
        self.daemon = subprocess.Popen(
            [str(TARGET / "yapd")], env=self.env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True
        )
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            if self.daemon.poll() is not None:
                self.fail(f"yapd exited early:\n{self.daemon.stdout.read()}")
            if subprocess.run(["busctl", "--user", "status", SERVICE], capture_output=True).returncode == 0:
                return
            time.sleep(0.1)
        self.fail("yapd never claimed com.yap.Yap on the session bus")

    def stop_daemon(self):
        if self.daemon and self.daemon.poll() is None:
            self.daemon.send_signal(signal.SIGTERM)
            self.assertEqual(self.daemon.wait(timeout=10), 0, "yapd should exit cleanly on SIGTERM")
        if self.daemon:
            self.daemon.stdout.close()
        self.daemon = None

    def call(self, method, signature="", *args, ok=True):
        command = ["busctl", "--user", "--json=short", "call", SERVICE, PATH, INTERFACE, method]
        if signature:
            command += [signature, *map(str, args)]
        result = subprocess.run(command, capture_output=True, text=True, timeout=10)
        if not ok:
            self.assertNotEqual(result.returncode, 0, f"{method} should be rejected")
            return result.stderr
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout)["data"]

    def state(self):
        return json.loads(self.call("Data")[0])

    def yapctl(self, *args):
        result = subprocess.run([str(TARGET / "yapctl"), *args], env=self.env, capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout

    def test_settings_and_snippets_persist_across_a_daemon_restart(self):
        self.assertIn("phase: idle", self.yapctl("status"))
        self.assertEqual(self.yapctl("cancel").strip().lower(), "idle")

        initial = self.state()
        self.assertEqual(initial["settings"]["cleanup_intensity"], "medium")
        self.assertEqual(initial["snippets"], [])

        settings = {"cleanup_intensity": "high", "audio_mode": "lower", "duck_level": 0.3, "trim_courtesy": False}
        updated = json.loads(self.call("UpdateSettings", "s", json.dumps(settings))[0])
        self.assertEqual(updated["settings"], settings)
        error = self.call("UpdateSettings", "s", json.dumps({**settings, "duck_level": 2}), ok=False)
        self.assertIn("duck_level", error)

        self.call("SaveSnippet", "tss", 0, "my address", "1 Example Street")
        saved = json.loads(self.call("SaveSnippet", "tss", 0, "sign off", "Cheers, Kyle")[0])
        self.assertEqual([s["trigger"] for s in saved["snippets"]], ["my address", "sign off"])
        address = next(s for s in saved["snippets"] if s["trigger"] == "my address")
        after_remove = json.loads(self.call("RemoveSnippet", "t", address["id"])[0])
        self.assertEqual([s["trigger"] for s in after_remove["snippets"]], ["sign off"])

        self.assertEqual(stat.S_IMODE(self.state_file.stat().st_mode), 0o600)
        self.stop_daemon()
        self.start_daemon()

        restored = self.state()
        self.assertEqual(restored["settings"], settings)
        self.assertEqual([(s["trigger"], s["expansion"]) for s in restored["snippets"]], [("sign off", "Cheers, Kyle")])


if __name__ == "__main__":
    unittest.main()
