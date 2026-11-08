import json
import math
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.error import HTTPError
from urllib.request import Request, urlopen

from budgeted_llm_proxy import BudgetedLlmProxy, cap_output_tokens, event_usage
from run_live_model_eval import evaluation_materials, metrics


class LiveModelEvalTest(unittest.TestCase):
    def setUp(self):
        self.received = []
        rows = self.received

        class Upstream(BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def do_POST(self):
                rows.append({"path": self.path, "headers": dict(self.headers), "body": json.loads(self.rfile.read(int(self.headers["Content-Length"])))})
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream")
                self.end_headers()
                self.wfile.write(b'data: {"usage":{"prompt_tokens":3,"completion_tokens":2,"total_tokens":5}}\n\n')
                self.wfile.write(b'data: [DONE]\n\n')

        self.upstream = ThreadingHTTPServer(("127.0.0.1", 0), Upstream)
        self.thread = threading.Thread(target=self.upstream.serve_forever, daemon=True)
        self.thread.start()

    def tearDown(self):
        self.upstream.shutdown()
        self.upstream.server_close()
        self.thread.join(timeout=3)

    def proxy(self, **options):
        return BudgetedLlmProxy(f"http://127.0.0.1:{self.upstream.server_address[1]}/v1", "upstream-secret",
                                max_calls=options.get("max_calls", 1), max_output_tokens=8,
                                timeout_seconds=options.get("timeout_seconds", 5), allow_local_http=True)

    def send(self, proxy, path="/chat/completions", token=None):
        request = Request(proxy.base_url + path, json.dumps({"model":"fixture","messages":[],"max_tokens":100}).encode(),
                          {"Content-Type":"application/json", "Authorization":"Bearer " + (token or proxy.token)})
        with urlopen(request, timeout=3) as response:
            return response.read()

    def test_call_budget_prevents_further_upstream_requests(self):
        with self.proxy() as proxy:
            self.assertIn(b"total_tokens", self.send(proxy))
            with self.assertRaises(HTTPError) as error:
                self.send(proxy)
            self.assertEqual(error.exception.code, 429)
            error.exception.close()
            self.assertEqual(proxy.forwarded_calls, 1)
            self.assertEqual(len(self.received), 1)
            self.assertEqual(self.received[0]["body"]["max_tokens"], 8)
            self.assertEqual(self.received[0]["headers"]["Authorization"], "Bearer upstream-secret")
            self.assertNotIn("upstream-secret", json.dumps(proxy.calls))
            self.assertEqual(proxy.calls[0]["usage"]["total_tokens"], 5)

    def test_expired_budget_and_wrong_credentials_do_not_reach_provider(self):
        with self.proxy() as proxy:
            with self.assertRaises(HTTPError) as error:
                self.send(proxy, token="wrong")
            self.assertEqual(error.exception.code, 401)
            error.exception.close()
            proxy.deadline = 0
            with self.assertRaises(HTTPError) as error:
                self.send(proxy)
            self.assertEqual(error.exception.code, 429)
            error.exception.close()
            self.assertEqual(self.received, [])

    def test_relay_cannot_be_used_for_other_endpoints(self):
        with self.proxy() as proxy:
            with self.assertRaises(HTTPError) as error:
                self.send(proxy, path="/admin")
            self.assertEqual(error.exception.code, 404)
            error.exception.close()
            self.assertEqual(self.received, [])

    def test_nonlocal_http_and_credentials_in_urls_are_rejected(self):
        for address in ("http://example.com/v1", "https://key@example.com/v1", "https://example.com/v1?key=secret"):
            with self.assertRaises(ValueError):
                BudgetedLlmProxy(address, "key", max_calls=1, max_output_tokens=1, timeout_seconds=1)
        for budget in (0, -1, math.nan, math.inf):
            with self.assertRaises(ValueError):
                self.proxy(timeout_seconds=budget)

    def test_caps_all_supported_protocol_token_fields_and_rejects_ambiguous_values(self):
        for key in ("max_tokens", "max_completion_tokens", "max_output_tokens"):
            body = {key: 100}
            cap_output_tokens(body, 8)
            self.assertEqual(body[key], 8)
        body = {"generationConfig":{"maxOutputTokens":100}}
        cap_output_tokens(body, 8)
        self.assertEqual(body["generationConfig"]["maxOutputTokens"], 8)
        for body in ({"max_tokens":True}, {"max_tokens":0}, {"max_tokens":1,"max_output_tokens":100}, {"generationConfig":[]}, {"generationConfig":{},"max_tokens":100000}):
            with self.assertRaises(ValueError):
                cap_output_tokens(body, 8)
        body = {}
        cap_output_tokens(body, 8, default_field="max_output_tokens")
        self.assertEqual(body, {"max_output_tokens":8})

    def test_native_usage_and_incomplete_or_failed_samples_have_honest_metrics(self):
        self.assertEqual(event_usage(b'{"response":{"usage":{"input_tokens":3}}}'), {"input_tokens":3})
        self.assertEqual(event_usage(b'{"usageMetadata":{"totalTokenCount":5}}'), {"totalTokenCount":5})
        for raw in (b"[DONE]", b"[]", b'{"response":"invalid"}'):
            self.assertIsNone(event_usage(raw))
        self.assertFalse(metrics([])["all_cases_passed"])
        result = metrics([{"success":True,"correction_calls":0}, {"success":True,"correction_calls":1}, {"success":False,"correction_calls":0}])
        self.assertEqual(result["case_count"], 3)
        self.assertAlmostEqual(result["first_pass_rate"], 1/3)
        self.assertAlmostEqual(result["validated_success_rate"], 2/3)
        materials = evaluation_materials()
        self.assertLess(len(materials["baseline"]), len(materials["extended"]))
        self.assertLess(len(materials["extended"]), len(materials["long"]))


if __name__ == "__main__":
    unittest.main()
