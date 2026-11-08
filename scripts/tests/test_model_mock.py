import json
import tempfile
import threading
import unittest
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import Request, urlopen

from mock_openai.protocol import MP4_1S_BASE64, PNG_1X1_BASE64
from mock_openai.transport import ProviderServer


class ModelMockBoundaryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory = tempfile.TemporaryDirectory()
        cls.server = ProviderServer(("127.0.0.1", 0), Path(cls.directory.name) / "requests.jsonl", PNG_1X1_BASE64, MP4_1S_BASE64, 8)
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()
        cls.url = "http://127.0.0.1:" + str(cls.server.server_address[1]) + "/v1/chat/completions"

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.thread.join(2)
        cls.directory.cleanup()

    def body(self):
        schema = {"type": "object", "required": ["ai_message", "conversation_state"], "properties": {"conversation_state": {"type": "object", "required": ["genre"]}}}
        contract = {"name": "inspiration", "schema": schema}
        return {"model": "a4n-virtual-text", "messages": [
            {"role": "system", "content": "小说概念大师·文思\n[A4N_OUTPUT_CONTRACT]\n" + json.dumps(contract) + "\n[/A4N_OUTPUT_CONTRACT]"},
            {"role": "user", "content": "开始创作"},
        ], "response_format": {"type": "json_schema", "json_schema": contract}}

    def post(self, body):
        request = Request(self.url, data=json.dumps(body).encode(), headers={"Content-Type": "application/json"})
        with urlopen(request, timeout=10) as response:
            return json.load(response)

    def test_real_http_rejects_missing_conflicting_contract_and_missing_input(self):
        self.assertIn("conversation_state", json.loads(self.post(self.body())["choices"][0]["message"]["content"]))
        for mutation in ("missing", "conflict", "input", "model"):
            body = self.body()
            if mutation == "missing":
                body["messages"][0]["content"] = "小说概念大师·文思"
            elif mutation == "conflict":
                body["response_format"]["json_schema"]["schema"] = {"type": "string"}
            elif mutation == "input":
                body["messages"][1]["content"] = " "
            else:
                body["model"] = " "
            with self.subTest(mutation=mutation), self.assertRaises(HTTPError) as caught:
                self.post(body)
            self.assertEqual(caught.exception.code, 422)
            caught.exception.close()

    def test_fault_output_reaches_recovery_instead_of_http_gate(self):
        body = self.body()
        body["model"] = "a4n-checkpoint-fail-inspiration"
        output = json.loads(self.post(body)["choices"][0]["message"]["content"])
        self.assertNotIn("genre", output["conversation_state"])

    def test_json_agent_http_fixture_uses_json_decisions_and_preserves_contract(self):
        contract={"name":"agent_json_compat","schema":{"type":"object","required":["action","arguments"],"properties":{"action":{"enum":["think","run_tool","final"]},"arguments":{"type":"object"}},"additionalProperties":False}}
        body={"model":"a4n-virtual-text","response_format":{"type":"json_object"},"messages":[
            {"role":"system","content":"你是 A4N 项目 Agent。\n当前能力域：content_optimization\n[A4N_OUTPUT_CONTRACT]\n"+json.dumps(contract,indent=2)+"\n[/A4N_OUTPUT_CONTRACT]"},
            {"role":"user","content":"A4N_OPTIMIZATION_PLAIN 何时写回？"},
        ]}
        message=self.post(body)["choices"][0]["message"]
        self.assertNotIn("tool_calls",message)
        self.assertEqual(json.loads(message["content"])["action"],"final")

    def test_bad_summary_is_returned_as_model_output_after_validating_the_request(self):
        contract={"name":"agent_context_summary","schema":{"type":"object","required":["pending","covered_through_sequence"]}}
        body={"model":"a4n-summary-fail-fields","response_format":{"type":"json_schema","json_schema":contract},"messages":[
            {"role":"system","content":"你负责压缩 A4N Agent 的历史\n[A4N_OUTPUT_CONTRACT]\n"+json.dumps(contract)+"\n[/A4N_OUTPUT_CONTRACT]"},
            {"role":"user","content":json.dumps({"previous_summary":None,"current_request":[],"messages":[],"covered_through_sequence":0})},
        ]}
        output=json.loads(self.post(body)["choices"][0]["message"]["content"])
        self.assertNotIn("pending",output)
        self.assertEqual(output["covered_through_sequence"],0)

    def test_system_contract_after_task_does_not_pollute_input_or_swallow_fault_output(self):
        contract={"name":"chapter_video_segment_plan","schema":{"type":"object","required":["segments"],"properties":{"segments":{"type":"array"}}}}
        payload={"current_shot":{"source_shot_id":"shot-2"},"constraints":{"selected_duration_secs":6}}
        body={"model":"a4n-checkpoint-fail-video-json","response_format":{"type":"json_schema","json_schema":contract},"messages":[
            {"role":"system","content":"你是影视分段导演；遵守 constraints.selected_duration_secs"},
            {"role":"user","content":"执行任务\n\n输入：\n"+json.dumps(payload)},
            {"role":"system","content":"[A4N_OUTPUT_CONTRACT]\n"+json.dumps(contract)+"\n[/A4N_OUTPUT_CONTRACT]"},
        ]}
        output=self.post(body)["choices"][0]["message"]["content"]
        with self.assertRaises(json.JSONDecodeError):json.loads(output)
        body["model"]="a4n-virtual-text"
        output=json.loads(self.post(body)["choices"][0]["message"]["content"])
        self.assertEqual(output["segments"][0]["source_shot_id"],"shot-2")

    def test_schema_and_unrelated_target_objects_are_not_adaptation_task_inputs(self):
        contract={"name":"chapter_media_adaptation_plan","schema":{"type":"object","required":["target","units"],"properties":{"target":{"enum":["manga","galgame"]},"units":{"type":"array","minItems":1}}}}
        payload={"target":"galgame","chapter":{"content":"#1 林岚在旧车站看到周衡。\n#2 林岚走进机房。"},"unrelated":{"target":{"shape":"object"}}}
        body={"model":"a4n-virtual-text","response_format":{"type":"json_schema","json_schema":contract},"messages":[
            {"role":"system","content":"你负责把原作章节改编为指定媒介的叙事计划"},
            {"role":"user","content":json.dumps(payload,ensure_ascii=False)},
            {"role":"system","content":"[A4N_OUTPUT_CONTRACT]\n"+json.dumps(contract)+"\n[/A4N_OUTPUT_CONTRACT]"},
        ]}
        output=json.loads(self.post(body)["choices"][0]["message"]["content"])
        self.assertEqual(output["target"],"galgame")
        self.assertEqual(output["units"][0]["source_span_ids"],["#1","#2"])

    def test_reference_discovery_fixture_explicitly_returns_both_required_arrays(self):
        contract={"name":"manga_page_reference_discovery","schema":{"type":"object","required":["schema_version","page_number","requirements","references"],"additionalProperties":False,"properties":{"schema_version":{"const":"manga_page_reference_discovery_v1"},"page_number":{"const":3},"requirements":{"type":"array"},"references":{"type":"array"}}}}
        task={"task":{"prompt_name":"manga_page_reference_discovery"},"input":{"current_page":{"page_number":3}},"output_contract":{"page_number":3}}
        body={"model":"a4n-virtual-text","response_format":{"type":"json_schema","json_schema":contract},"messages":[
            {"role":"user","content":json.dumps({"input":{"available_assets":[]}})+"\n\n输入：\n"+json.dumps(task)},
            {"role":"system","content":"[A4N_OUTPUT_CONTRACT]\n"+json.dumps(contract)+"\n[/A4N_OUTPUT_CONTRACT]"},
        ]}
        output=json.loads(self.post(body)["choices"][0]["message"]["content"])
        self.assertEqual(output["page_number"],3)
        self.assertEqual(output["requirements"],[])
        self.assertEqual(output["references"],[])

    def test_fact_fault_keeps_its_calendar_context_for_normal_contract_validation(self):
        contract={"name":"object_discovery","schema":{"type":"object","required":["objects"],"properties":{"objects":{"type":"array"}}}}
        body={"model":"a4n-fact-feedback-invalid","response_format":{"type":"json_schema","json_schema":contract},"messages":[
            {"role":"user","content":json.dumps({"task":{"name":"object_discovery"},"chapter":{"content":"#1 记忆城每年有六个季节。"}},ensure_ascii=False)},
            {"role":"system","content":"[A4N_OUTPUT_CONTRACT]\n"+json.dumps(contract)+"\n[/A4N_OUTPUT_CONTRACT]"},
        ]}
        self.assertEqual(json.loads(self.post(body)["choices"][0]["message"]["content"]),{"objects":[]})
        contract={"name":"project_fact_consolidation","schema":{"type":"object","required":["changes"],"properties":{"changes":{"type":"array"}}}}
        body["response_format"]["json_schema"]=contract
        body["messages"][0]["content"]=json.dumps({"task":{"name":"project_fact_consolidation"},"blueprint":{"id":1,"world_setting":{"culture_and_society":{"calendar":"旧历法"}}},"part_outlines":[],"chapters":[{"chapter_number":9,"content":"#1 记忆城每年有六个季节。"}]},ensure_ascii=False)
        body["messages"][1]["content"]="[A4N_OUTPUT_CONTRACT]\n"+json.dumps(contract)+"\n[/A4N_OUTPUT_CONTRACT]"
        output=json.loads(self.post(body)["choices"][0]["message"]["content"])
        self.assertEqual(output["changes"][0]["source_span_ids"],["#999999"])


if __name__ == "__main__":
    unittest.main()
