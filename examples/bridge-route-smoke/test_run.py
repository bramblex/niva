import subprocess
import unittest

from run import stop_process


class FakeProcess:
    pid = 7123

    def __init__(self, events, wait_outcomes):
        self.events = events
        self.wait_outcomes = list(wait_outcomes)
        self.returncode = None

    def poll(self):
        return self.returncode

    def wait(self, timeout):
        self.events.append(("wait", timeout))
        outcome = self.wait_outcomes.pop(0)
        if isinstance(outcome, BaseException):
            raise outcome
        return outcome

    def terminate(self):
        self.events.append(("terminate", self.pid))
        self.returncode = 1

    def kill(self):
        self.events.append(("kill", self.pid))
        self.returncode = 1


class StopProcessTests(unittest.TestCase):
    def test_windows_posts_close_to_runner_pid_and_waits_before_termination(self):
        events = []
        process = FakeProcess(events, [0])

        def close_windows(pid):
            events.append(("wm_close", pid))
            process.returncode = 0

        stop_process(process, platform="win32", close_windows=close_windows)

        self.assertEqual(events, [("wm_close", 7123), ("wait", 5)])

    def test_windows_terminates_only_runner_process_after_grace_timeout(self):
        events = []
        process = FakeProcess(events, [subprocess.TimeoutExpired("niva", 5), 1])

        stop_process(process, platform="win32", close_windows=lambda pid: events.append(("wm_close", pid)))

        self.assertEqual(events, [
            ("wm_close", 7123), ("wait", 5), ("terminate", 7123), ("wait", 5),
        ])

    def test_windows_close_error_still_reaches_targeted_fallback(self):
        events = []
        process = FakeProcess(events, [subprocess.TimeoutExpired("niva", 5), 1])

        def close_windows(pid):
            events.append(("wm_close", pid))
            raise RuntimeError("mock user32 failure")

        stop_process(process, platform="win32", close_windows=close_windows)

        self.assertEqual(events, [
            ("wm_close", 7123), ("wait", 5), ("terminate", 7123), ("wait", 5),
        ])

    def test_windows_kills_runner_process_only_after_terminate_timeout(self):
        events = []
        process = FakeProcess(events, [
            subprocess.TimeoutExpired("niva", 5),
            subprocess.TimeoutExpired("niva", 5),
            1,
        ])

        stop_process(process, platform="win32", close_windows=lambda _pid: None)

        self.assertEqual(events, [
            ("wait", 5), ("terminate", 7123), ("wait", 5), ("kill", 7123), ("wait", 5),
        ])

    def test_macos_keeps_existing_terminate_then_kill_behavior(self):
        events = []
        process = FakeProcess(events, [subprocess.TimeoutExpired("niva", 5), 1])

        def unexpected_close(_pid):
            self.fail("macOS cleanup must not use the Windows window-close path")

        stop_process(process, platform="darwin", close_windows=unexpected_close)

        self.assertEqual(events, [("terminate", 7123), ("wait", 5), ("kill", 7123), ("wait", 5)])


if __name__ == "__main__":
    unittest.main()
