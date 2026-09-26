"""Measure saved inputs; reconstruction counts characters/bytes, never estimated tokens."""

import csv
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
LEGACY = ROOT / "data/prompt-context-audit-20260926/legacy"


def compact(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def comparison(before, after):
    return {
        "before_chars": len(before), "after_chars": len(after),
        "before_bytes": len(before.encode()), "after_bytes": len(after.encode()),
        "chars_removed_percent": round((1 - len(after) / len(before)) * 100, 2),
    }


csv.field_size_limit(64 * 1024 * 1024)
with (LEGACY / "all_packs.csv").open() as stream:
    packs = list(csv.reader(stream))
with (LEGACY / "traces3.csv").open() as stream:
    traces = list(csv.reader(stream))
source = json.loads((LEGACY / "pack_425.txt").read_text())
before = (LEGACY / "pack_425.txt").read_text().rstrip("\n")
source_after = compact({"content": source["content"]})
cursor = 0
marked = []
for span in source["source_spans"]:
    start = source["content"].find(span["text"], cursor)
    assert start >= cursor, "Legacy span order no longer matches original text"
    end = start + len(span["text"])
    marked.extend([source["content"][cursor:start], "⟦" + span["id"] + "⟧", span["text"]])
    cursor = end
marked.append(source["content"][cursor:])
report = {
    "evidence_kind": "saved-pack measurements and deterministic representation reconstruction",
    "legacy_input_sha256": hashlib.sha256((LEGACY / "all_packs.csv").read_bytes()).hexdigest(),
    "trace_rows_with_usage": len(traces),
    "historical_input_tokens": sum(int(row[3]) for row in traces),
    "stored_pack_count": len(packs),
    "stored_pack_corpus_compaction": comparison("".join(row[1] for row in packs),
                                                "".join(compact(json.loads(row[1])) for row in packs)),
    "video_source_pack_whole_chapter_upper_bound": comparison(before, source_after),
    "source_representation_with_ids": comparison(
        compact({"content": source["content"], "source_spans": source["source_spans"]}),
        compact({"content": "".join(marked)})),
    "limitations": [
        "Corpus compaction measures stored packs, not full wire requests or billable tokens.",
        "Video source reconstruction keeps the whole chapter; current task-specific paragraph selection may be shorter.",
        "Historical source spans lack offsets; annotation is reconstructed in verified source order.",
        "No remote model quality or cache hit-rate A/B was performed.",
    ],
}
output = Path(__file__).with_name("measurements.json")
output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
print(json.dumps(report, ensure_ascii=False, indent=2))
