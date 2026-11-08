"""Independent required execution IDs for model acceptance reports."""
from __future__ import annotations
from model_contract_vectors import vectors
from check_model_call_coverage import structured_ids


def contract_cases(ledger:dict,corpus:dict)->list[dict]:
    required=structured_ids(ledger)|{row["call_id"] for row in ledger.get("request_contracts",[])}
    if set(corpus)!=required:
        raise ValueError(f"生产合同导出不完整；缺少 {sorted(required-set(corpus))}；多出 {sorted(set(corpus)-required)}")
    result=[]
    for call_id,row in sorted(corpus.items()):
        result.extend(vectors(call_id,row["schema"],row.get("example")))
    return result


def adapter_case_ids(ledger:dict,inventory:dict)->set[str]:
    result=set()
    for row in inventory["catalogs"]["text"]:
        for scenario in ledger["shared_scenarios"]["text"]:
            result.add(f"llm.transport/{row['id']}/json/{scenario}")
        for scenario in ledger["shared_scenarios"].get("tools",[]):
            result.add(f"llm.transport/{row['id']}/tools/{scenario}")
    for row in inventory["catalogs"]["image"]:
        backend=row["id"]
        for count in (0,1,2):
            scenarios=("actual-request-parameters","decodable-artifact") if count==0 or row["references"] else ("unsupported-before-request",)
            result.update(f"image.generate/{backend}/refs-{count}/{scenario}" for scenario in scenarios)
        for scenario in ledger["shared_scenarios"]["image"]:
            if scenario in {"actual-request-parameters","decodable-artifact","unsupported-before-request"}:continue
            variant="invalid-input" if scenario in {"empty-prompt","bad-width","empty-model"} or scenario.startswith("reference-") else "text"
            result.add(f"image.generate/{backend}/{variant}/{scenario}")
    for row in inventory["catalogs"]["video"]:
        backend=row["id"]
        for mode in ("text","first","first-last","references","first-and-references","video-audio"):
            references=2 if mode=="references" else 1 if mode=="first-and-references" else 0
            supported=(mode!="first-last" or row["last_frame"]) and (row["max_reference_images"] is None or references<=row["max_reference_images"]) and (mode!="first-and-references" or row["first_frame_with_references"]) and (mode!="video-audio" or row["reference_video_audio"])
            scenarios=("actual-request-parameters","real-artifact") if supported else ("unsupported-before-request",)
            result.update(f"video.generate/{backend}/{mode}/{scenario}" for scenario in scenarios)
        for scenario in ledger["shared_scenarios"]["video"]:
            if scenario in {"actual-request-parameters","real-artifact","unsupported-before-request"}:continue
            if not row["asynchronous"] and scenario in {"unknown-status","pending","failed"}:continue
            variant="job" if scenario in {"delete-capability","empty-job-before-request"} else "invalid-input" if scenario in {"prompt","model","width","fps","duration"} else "fault"
            result.add(f"video.generate/{backend}/{variant}/{scenario}")
    for scenario in ledger["shared_scenarios"]["embedding"]:
        variant="batch" if scenario in {"order-and-identity","actual-request-order"} else "invalid-input" if scenario in {"batch","model"} else "response"
        result.add(f"embedding.embed/api/{variant}/{scenario}")
    for row in inventory["catalogs"].get("search",[]):
        for scenario in ledger["shared_scenarios"].get("search",[]):
            if scenario in {"missing-finish","truncated"} and row["id"] not in {"anthropic","openai_responses"}:continue
            result.add(f"web_search/{row['id']}/query/{scenario}")
    # Configuration dimension failure and response dimension failure are distinct.
    result.add("embedding.embed/api/invalid-input/dimension")
    result.add("video.artifact/ffprobe/valid/actual-metadata")
    result.update(f"video.artifact/ffprobe/invalid/{scenario}" for scenario in ("empty","fake","truncated","audio-only"))
    return result


def required_case_ids(ledger:dict,inventory:dict,corpus:dict,profile:str)->set[str]:
    result=adapter_case_ids(ledger,inventory)
    for row in ledger.get("free_text",[]):
        scenarios=row.get("scenarios",("valid","empty","over-limit") if row["call_id"]=="chapter_visual_plan" else ("valid","empty","structured","fenced","thinking"))
        result.update(f"{row['call_id']}/runtime-text/output/{scenario}" for scenario in scenarios)
        result.update(f"{row['call_id']}/runtime-text/media/{scenario}" for scenario in row.get("media_scenarios",[]))
    requests={row["call_id"] for row in ledger.get("request_contracts",[])}
    boundaries={call_id:row["boundary"] for row in ledger["structured"] if "boundary" in row for call_id in row["calls"]}
    for case in contract_cases(ledger,corpus):
        result.add(case["case_id"]+"/rust")
        result.add(case["case_id"]+"/ajv")
        key=case["case_id"].split("/contract/",1)[1]
        if case["call_id"] in requests:
            result.add(f"{case['call_id']}/runtime-request/{key}/before-http")
        elif case["call_id"] in boundaries:
            boundary=boundaries[case["call_id"]]
            result.add(f"{case['call_id']}/{boundary['adapter']}/{key}/{boundary['scenario']}")
        else:
            result.add(f"{case['call_id']}/runtime-schema/{key}/recovery-enforcement")
    for call_id in structured_ids(ledger):
        result.update(f"{call_id}/{preset['id']}/business-schema/actual-wire-roundtrip" for preset in inventory["catalogs"]["text"])
        if call_id in boundaries:continue
        result.add(f"{call_id}/runtime-schema/correction/valid-replacement")
        result.add(f"{call_id}/runtime-schema/cancellation/no-late-adoption")
        result.update(f"{call_id}/runtime-schema/{reason}/terminal-before-validation" for reason in ("missing-finish","length","content_filter"))
    if profile=="model-full":
        result.update(f"rust-tests/{crate}/{prefix}" for crate,prefixes in ledger["semantic_rust_groups"].items() for prefix in prefixes)
        result.add("creation/full-cli-browser")
    elif profile!="model-fast":raise ValueError("unknown model acceptance profile")
    result.add("python/model-assurance")
    return result
