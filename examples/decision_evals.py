#!/usr/bin/env python3
"""Synthetic saved-score eval. No API calls, real traces, orders or provider prices."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess


def write(path, value):
    data = (json.dumps(value, indent=2, allow_nan=False) + "\n").encode()
    with path.open("xb") as output:
        output.write(data)
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("out", type=Path, help="Fresh private fixture directory")
    parser.add_argument("--paperhands", default="paperhands")
    args = parser.parse_args()
    args.out.mkdir(mode=0o700)
    root = args.out.resolve()
    cases = []
    for fold in range(5):
        for index, failure in enumerate([False, False, True, True]):
            cases.append(dict(id=f"trace-{fold}-{index}", group=f"event-{fold}",
                              fold=fold, text="Synthetic source-claim trace; not market evidence.",
                              failure=failure))
    dataset = dict(schema="paperhands.judge-dataset/v1", id="synthetic-source-claims",
                   data_use="synthetic", folds=5, cases=cases)
    dataset_hash = write(root / "dataset.json", dataset)
    rubric = dict(id="unsupported-source", failure="Does the trace claim a fact its supplied source does not support?")
    paths = []
    for identity, positive, negative, rate in [
        ("low-range", 0.4, 0.1, 1), ("high-range", 0.9, 0.7, 4),
        ("uninformative", 0.4, 0.4, 1),
    ]:
        scores = dict(schema="paperhands.judge-scores/v1", id=identity,
                      dataset_sha256=dataset_hash, provider="synthetic-fixture", model=identity,
                      rubric=rubric, pricing=dict(input_usd_per_million=rate,
                      output_usd_per_million=0, source="Illustrative fixture rate; not provider pricing"),
                      cases=[dict(id=c["id"], status="scored", score=positive if c["failure"] else negative,
                                  usage=dict(input_tokens=100, output_tokens=0), wall_time_ms=1)
                             for c in cases])
        path = root / f"{identity}.json"
        write(path, scores)
        paths.append(path)
    plan = dict(schema="paperhands.judge-plan/v1", id="example-unexecuted-jev",
                model="jev-1.13.0", dataset=dict(path="dataset.json", sha256=dataset_hash),
                rubric=rubric, pricing=None, limits=dict(max_calls=20, wall_time_ms=60000,
                request_timeout_ms=5000, max_total_request_bytes=1048576,
                max_response_bytes=65536, max_total_response_bytes=1048576))
    write(root / "unexecuted-judge-plan.json", plan)
    argv = [args.paperhands, "judge", "analyze", "--dataset", str(root / "dataset.json"),
            "--out", str(root / "analysis"), "--min-precision", "0.8", "--min-recall", "0.8"]
    for path in paths:
        argv += ["--scores", str(path)]
    result = subprocess.run(argv, check=True, capture_output=True, text=True)
    report = json.loads(result.stdout)["data"]["result"]
    low, high, flat = report["models"]
    assert low["default_validation"]["recall"] == 0
    assert low["tuned_validation"]["f_beta"] == 1
    assert all(f["training_selected_threshold"] == 0.4 for f in low["folds"])
    assert high["validation_eligible"]
    assert not flat["validation_eligible"] and flat["tuned_validation"] is None
    assert all(f["no_qualifying_threshold"] for f in flat["folds"])
    assert report["cost_quality_frontier"] == ["low-range"]
    assert report["protocol"]["model_calls"] == 0
    print(json.dumps(dict(fixture=str(root), cases=len(cases), models=3, folds=5,
                         model_calls=0, financial_evidence=False,
                         next=["paperhands", "view", "judge", "--dir", str(root / "analysis")]), indent=2))


if __name__ == "__main__":
    main()
