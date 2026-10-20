#!/usr/bin/env python3
"""Create an explicitly synthetic research companion in a fresh project.

Standard-library only. Runs local Paperhands operations with exact argv; no
network, venue, external model service, evaluator or orders. Notes and questions
use the binary's bundled local neural encoder. Existing directories are refused.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--paperhands", default="paperhands")
    args = parser.parse_args()
    binary = shutil.which(args.paperhands)
    if not binary:
        raise SystemExit("Paperhands executable not found")
    project = args.out.absolute()
    project.mkdir()
    env = dict(os.environ, PAPERHANDS_SKILLS_DIR=str(project / ".paperhands/managed-skill"))
    env.pop("PAPERHANDS_WORKSPACE", None)
    commands = []

    def run(*argv):
        out = subprocess.run([binary, *argv], cwd=project, env=env,
                             capture_output=True, text=True, timeout=30)
        commands.append({"argv": list(argv), "exit_code": out.returncode})
        if out.returncode:
            raise RuntimeError(out.stderr or out.stdout)
        return json.loads(out.stdout)["data"]

    run("init", "--name", "Illustrative research studio")
    notes = {}
    for key, kind, title, text in [
        ("direction", "direction", "Hold hypotheses lightly", "Illustrative direction: observe mechanisms, generate competing explanations, then run a cheap bounded test. Preserve unknowns and useful failures."),
        ("question", "question", "What replenishes a queue?", "Illustrative open question: could returning liquidity reflect inventory targets, independent arrivals, or an auction clock? No market observation is asserted."),
        ("inventory", "lesson", "Do not lose the residual", "Illustrative accounting question: retain every unpriced holding. A missing exit is unknown, never zero. This is a method note, not a measured trading result."),
        ("analogy", "hypothesis", "Borrow from an auction", "Illustrative untested analogy: auction timing may help distinguish queue mechanisms. Name where the analogy breaks before testing it."),
    ]:
        added = run("memory", "add", "--kind", kind, "--title", title,
                    "--text", text, "--source", "synthetic:research-desk-example")
        notes[key] = "memory:" + added["record"]["id"]
    for key, title, kind, dependencies in [
        ("map", "Record competing explanations", "plan", []),
        ("control", "Choose a discriminating observation", "experiment", ["map"]),
        ("inventory", "Audit unknown holdings", "review", []),
        ("wander", "Explore a mechanism outside this family", "research", []),
    ]:
        argv = ["tasks", "add", "--id", key, "--title", title, "--kind", kind]
        for dependency in dependencies:
            argv += ["--depends-on", dependency]
        run(*argv)
    run("tasks", "update", "--id", "map", "--state", "done", "--note-text",
        "Three untested explanations are retained in the synthetic question note. Next: a discriminating control; no financial conclusion.")
    graph = run("graph", "inspect")
    tasks = {node["logical_id"]: node["id"] for node in graph["nodes"]
             if node.get("attributes", {}).get("current")}
    for source, target, priority in [(notes["question"], tasks["control"], "0.7"),
                                     (notes["inventory"], tasks["inventory"], "0.9"),
                                     (notes["analogy"], notes["question"], "0.8")]:
        run("graph", "link", "--from", source, "--to", target,
            "--relation", "informs", "--priority", priority,
            "--evidence-text", "Declared attention for this synthetic demonstration; no claim of verified causation or independent evidence.")
    focus = run("ask", "What is our focus?")
    next_work = run("ask", "What should I do next?")
    clusters = run("graph", "clusters")
    view = run("view", "graph", "--dir", str(project / ".paperhands"))
    (project / ".paperhands/example-commands.json").write_text(json.dumps(commands, indent=2))
    print(json.dumps({"project": str(project), "synthetic": True,
                      "focus": focus["answer"], "next": next_work["answer"],
                      "objects": view["coverage"]["total"],
                      "clusters": len(clusters["groups"]),
                      "snapshot_sha256": view["snapshot_sha256"],
                      "mcp": run("mcp-config")["mcpServers"],
                      "executed_evaluators_or_orders": False}, indent=2))


if __name__ == "__main__":
    main()
