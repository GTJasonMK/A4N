"""Confirm the slow-polling defect in the real component, without any backend writes."""
import functools
import http.server
import json
from pathlib import Path
import threading

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / ".dev-runtime/project-optimization-20260927"


class Handler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *_):
        pass


with http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(Handler, directory=str(OUTPUT))) as server:
    threading.Thread(target=server.serve_forever, daemon=True).start()
    try:
        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(executable_path="/usr/bin/chromium")
            pages = {}
            errors = []
            for delay in [50, 1800]:
                page = browser.new_page(viewport={"width": 1100, "height": 720})
                page.on("pageerror", lambda error: errors.append(str(error)))
                page.goto(f"http://127.0.0.1:{server.server_port}/?delay={delay}")
                page.wait_for_selector('[role="status"]')
                pages[delay] = page
            pages[1800].wait_for_function("window.calls.filter(c => c.completed && c.id > 1).length >= 3")
            result = {}
            for delay, page in pages.items():
                button = page.get_by_role("button", name="novel.factFeedback.consolidate", exact=True)
                result[delay] = {
                    "coverage": page.locator('[role="status"]').inner_text(),
                    "action_disabled": button.is_disabled(),
                    "calls": page.evaluate("window.calls"),
                }
                page.screenshot(path=str(OUTPUT / f"latency-{delay}.png"))
            assert not errors, errors
            assert '"covered":1' in result[50]["coverage"] and not result[50]["action_disabled"]
            assert '"covered":0' in result[1800]["coverage"] and result[1800]["action_disabled"]
            browser.close()
        (OUTPUT / "result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2))
        print("DEFECT CONFIRMED: completed responses are ignored at 1800ms; 50ms control updates normally.")
        print(json.dumps(result, ensure_ascii=False))
    finally:
        server.shutdown()
