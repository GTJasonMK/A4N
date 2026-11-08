import copy
import unittest
from model_contract_vectors import judge, sample, vectors


class ContractVectorsTests(unittest.TestCase):
    def setUp(self):
        self.schema = {"type":"object","required":["id","exit_state"],"additionalProperties":False,"properties":{
            "id":{"type":"integer","minimum":1},"exit_state":{"type":"object"},"note":{"type":["string","null"]},
            "items":{"type":"array","minItems":1,"items":{"type":"object","required":["name"],"properties":{"name":{"type":"string","minLength":1}}}}}}

    def test_generates_nested_missing_type_null_empty_and_optional_cases(self):
        cases = vectors("scene",self.schema)
        ids = {case["case_id"] for case in cases}
        for suffix in ["/exit_state:missing","/exit_state:type-null","/id:type-string","/items/0/name:missing","/items/0/name:empty-string","/note:optional-absent"]:
            self.assertTrue(any(case_id.endswith(suffix) for case_id in ids),suffix)
        self.assertEqual(cases[0]["value"]["exit_state"],{})
        self.assertEqual(len(ids),len(cases))

    def test_missing_skipped_duplicate_and_wrong_actual_results_cannot_pass(self):
        expected = vectors("scene",self.schema)
        actual = [{"case_id":case["case_id"],"accepted":case["expected"]} for case in expected]
        judge(expected,actual)
        for changed in [actual[:-1],actual+actual[:1],[]]:
            with self.assertRaises(ValueError): judge(expected,changed)
        actual[0]["accepted"] = None
        with self.assertRaises(ValueError): judge(expected,actual)

    def test_tests_are_sensitive_to_validator_bypass_and_constraint_removal(self):
        expected = vectors("scene",self.schema)
        with self.assertRaises(ValueError): judge(expected,[{"case_id":case["case_id"],"accepted":True} for case in expected])
        actual = [{"case_id":case["case_id"],"accepted":case["expected"]} for case in expected]
        case = next(case for case in actual if case["case_id"].endswith("/exit_state:missing"))
        case["accepted"] = True
        with self.assertRaisesRegex(ValueError,"exit_state:missing"): judge(expected,actual)

    def test_resolves_local_definitions_without_accepting_remote_refs(self):
        schema = {"$ref":"#/$defs/record","$defs":{"record":self.schema}}
        self.assertIn("exit_state",sample(schema))
        with self.assertRaises(ValueError): sample({"$ref":"https://example.invalid/schema"})

    def test_conditional_required_fields_are_not_mislabeled_as_optional(self):
        schema = copy.deepcopy(self.schema)
        schema["allOf"] = [{"if":{"properties":{"id":{"const":1}}},"then":{"required":["note"]}}]
        cases = vectors("conditional",schema,{"id":1,"exit_state":{},"note":"text","items":[]})
        note_cases = [case for case in cases if case.get("path") == "/note"]
        self.assertTrue(any(case["rule"] == "missing" and not case["expected"] for case in note_cases))
        self.assertFalse(any(case["rule"] == "optional-absent" for case in note_cases))

    def test_tagged_union_uses_the_actual_variant_fields(self):
        schema={"oneOf":[{"type":"object","properties":{"type":{"const":"none"}},"required":["type"]},{"type":"object","properties":{"type":{"const":"frames"},"first":{"type":"string"}},"required":["type","first"]}]}
        cases=vectors("video-media",schema,{"type":"frames","first":"image"})
        self.assertTrue(any(case["case_id"].endswith("/first:missing") for case in cases))

    def test_large_float_mutations_actually_cross_representable_bounds(self):
        schema={"type":"number","minimum":-3.4028234663852886e38,"maximum":3.4028234663852886e38}
        cases=vectors("float",schema)
        self.assertLess(next(case["value"] for case in cases if case["rule"]=="minimum"),schema["minimum"])
        self.assertGreater(next(case["value"] for case in cases if case["rule"]=="maximum"),schema["maximum"])


if __name__ == "__main__": unittest.main()
