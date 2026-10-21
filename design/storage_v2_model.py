#!/usr/bin/env python3
"""Offline storage-design model, NOT the Paperhands store or a cloud adapter.

Run: python3 -B design/storage_v2_model.py
Flat in-memory sets stand in for paged manifests; sequential CAS stands in for a
backend's atomic conditional write. This checks merge rules, not cloud behavior,
performance, authentication, crash durability or distributed execution.
"""
import hashlib
import json
import math
import random
import unittest


def encode(value):
    # Fixture encoding only; hashes always cover bytes, not semantic JSON identity.
    return json.dumps(value, sort_keys=True, separators=(",", ":"),
                      allow_nan=False).encode("utf-8")


def digest(data):
    return "sha256:" + hashlib.sha256(data).hexdigest()


class IntegrityError(ValueError):
    pass


class StaleHead(ValueError):
    pass


class Pool:
    def __init__(self):
        self.objects = {}

    def put(self, data):
        oid = digest(data)
        if oid in self.objects and self.read(oid) != data:
            raise IntegrityError("immutable object collision")
        self.objects.setdefault(oid, data)
        return oid

    def read(self, oid):
        if oid not in self.objects or digest(self.objects[oid]) != oid:
            raise IntegrityError("missing or changed object: " + oid)
        return self.objects[oid]

    def value(self, oid):
        def unique(pairs):
            value = {}
            for key, item in pairs:
                if key in value:
                    raise IntegrityError("duplicate typed JSON key")
                value[key] = item
            return value

        def nonfinite(_):
            raise IntegrityError("nonfinite typed JSON number")

        def finite_float(number):
            value = float(number)
            if not math.isfinite(value):
                raise IntegrityError("nonfinite typed JSON number")
            return value

        try:
            text = self.read(oid).decode("utf-8")
        except UnicodeDecodeError as error:
            raise IntegrityError("typed JSON must use UTF-8") from error
        return json.loads(text, object_pairs_hook=unique,
                          parse_constant=nonfinite, parse_float=finite_float)


def revision(pool, entity, writer, operation, value, parents=(), provenance=()):
    payload = pool.put(encode(value))
    return pool.put(encode({
        "schema": "paperhands.revision/v2", "project_id": entity.split("/")[0],
        "entity_id": entity, "kind": "task", "writer_id": writer,
        "operation_id": operation, "parents": sorted(set(parents)),
        "payload": payload, "provenance": sorted(set(provenance)),
        "recorded_at_ms": 0,
    }))


def heads(pool, project, facts):
    """Verify retained revision closure; project maximal causal revisions."""
    records = {oid: pool.value(oid) for oid in facts}
    operations, current = {}, {}
    for oid, record in records.items():
        if (record["schema"] != "paperhands.revision/v2"
                or record["project_id"] != project
                or not record["entity_id"].startswith(project + "/")):
            raise IntegrityError("project/schema mismatch")
        pool.read(record["payload"])
        for source in record["provenance"]:
            pool.read(source)
        key = (record["writer_id"], record["operation_id"])
        if key in operations and operations[key] != oid:
            raise IntegrityError("operation reused with different bytes")
        operations[key] = oid
        current.setdefault(record["entity_id"], set()).add(oid)
        for parent in record["parents"]:
            if parent not in records:
                raise IntegrityError("missing causal parent")
            if records[parent]["entity_id"] != record["entity_id"]:
                raise IntegrityError("parent belongs to another entity")
    # Depth-first cycle detection is deliberately bounded to this tiny fixture.
    visited, visiting = set(), set()

    def visit(oid):
        if oid in visiting:
            raise IntegrityError("revision ancestry cycle")
        if oid in visited:
            return
        visiting.add(oid)
        for parent in records[oid]["parents"]:
            visit(parent)
        visiting.remove(oid)
        visited.add(oid)

    for oid in records:
        visit(oid)
    for record in records.values():
        current[record["entity_id"]].difference_update(record["parents"])
    return {entity: frozenset(ids) for entity, ids in current.items()}


def merge(*snapshots):
    return frozenset().union(*snapshots)


def cyclic_edges(edges):
    """All edges internal to cyclic components; no arrival-order tie breaker."""
    outgoing = {}
    for source, target in edges:
        outgoing.setdefault(source, set()).add(target)

    def reaches(start, target):
        pending, seen = [start], set()
        while pending:
            node = pending.pop()
            if node == target:
                return True
            if node not in seen:
                seen.add(node)
                pending.extend(outgoing.get(node, ()))
        return False

    return frozenset((a, b) for a, b in edges if reaches(b, a))


class Ref:
    """Sequential conditional-head model; NOT a concurrent storage primitive."""
    def __init__(self, pool, project):
        self.pool, self.project = pool, project
        self.token, self.commit, self.facts = None, None, frozenset()
        self.generation = 0

    def publish(self, facts, expected):
        heads(self.pool, self.project, facts)  # Reject missing closure before CAS.
        if expected != self.token:
            raise StaleHead("precondition failed; read head and merge again")
        if not self.facts <= facts:
            raise IntegrityError("ref update loses retained facts")
        commit = self.pool.put(encode({
            "schema": "paperhands.snapshot-model/v2", "project_id": self.project,
            "parents": [self.commit] if self.commit else [],
            "facts": sorted(facts),  # Production uses Merkle roots, not this list.
        }))
        self.generation += 1
        self.token = "opaque-fixture-etag-" + str(self.generation)
        self.commit, self.facts = commit, frozenset(facts)
        return commit


class StorageContract(unittest.TestCase):
    def setUp(self):
        self.pool = Pool()
        self.entity = "p/origin/task"
        self.base = revision(self.pool, self.entity, "w0", "start", {"state": "open"})
        self.base_facts = frozenset([self.base])

    def test_identical_content_keeps_distinct_receipts_and_unknowns(self):
        raw = self.pool.put(b"same raw observation")
        self.assertEqual(raw, self.pool.put(b"same raw observation"))
        value = {"net": None, "unpriced_shares": "11.86"}
        a = revision(self.pool, "p/a/receipt", "a", "1", value, provenance=[raw])
        b = revision(self.pool, "p/b/receipt", "b", "1", value, provenance=[raw])
        self.assertNotEqual(a, b)
        self.assertEqual(self.pool.value(a)["payload"], self.pool.value(b)["payload"])
        self.assertEqual(2, len(heads(self.pool, "p", merge([a], [b]))))
        self.assertIsNone(self.pool.value(self.pool.value(a)["payload"])["net"])

    def test_invalid_legacy_json_stays_opaque_and_cannot_activate_as_typed_data(self):
        for raw in [b'{"kind":"original","kind":"replacement"}', b'{"score":NaN}']:
            oid = self.pool.put(raw)
            self.assertEqual(raw, self.pool.read(oid))
            with self.assertRaises(IntegrityError):
                self.pool.value(oid)

    def test_overflowing_exponents_remain_opaque_and_refuse_typed_activation(self):
        for raw in [b'{"score":1e999}', b'{"score":-1e999}',
                    b'{"scores":[1e999]}']:
            with self.subTest(raw=raw):
                oid = self.pool.put(raw)
                self.assertEqual(raw, self.pool.read(oid))
                with self.assertRaises(IntegrityError):
                    self.pool.value(oid)
        for number in ["1.7976931348623157e308", "-1.7976931348623157e308", "6.25e-2"]:
            raw = ('{"score":' + number + '}').encode("utf-8")
            self.assertEqual(float(number), self.pool.value(self.pool.put(raw))["score"])

    def test_typed_metadata_requires_utf8_without_encoding_autodetection(self):
        text = '{"kind":"caf\u00e9"}'
        for encoding in ["utf-16", "utf-16-le", "utf-16-be",
                         "utf-32", "utf-32-le", "utf-32-be"]:
            with self.subTest(encoding=encoding):
                raw = text.encode(encoding)
                oid = self.pool.put(raw)
                self.assertEqual(raw, self.pool.read(oid))
                with self.assertRaises((IntegrityError, json.JSONDecodeError)):
                    self.pool.value(oid)
        raw = b'\xff{"kind":"opaque"}'
        oid = self.pool.put(raw)
        self.assertEqual(raw, self.pool.read(oid))
        with self.assertRaises(IntegrityError):
            self.pool.value(oid)
        self.assertEqual({"kind": "caf\u00e9"}, self.pool.value(self.pool.put(text.encode("utf-8"))))

    def test_tampered_payload_refuses(self):
        payload = self.pool.value(self.base)["payload"]
        self.pool.objects[payload] = b"changed"
        with self.assertRaises(IntegrityError):
            heads(self.pool, "p", self.base_facts)

    def test_missing_provenance_refuses(self):
        a = revision(self.pool, self.entity, "w1", "change", {},
                     [self.base], [digest(b"not uploaded")])
        with self.assertRaises(IntegrityError):
            heads(self.pool, "p", merge(self.base_facts, [a]))

    def test_concurrent_decisions_need_resolution_and_can_reopen(self):
        a = revision(self.pool, self.entity, "a", "1", {"state": "done"}, [self.base])
        b = revision(self.pool, self.entity, "b", "1", {"state": "blocked"}, [self.base])
        facts = merge(self.base_facts, [a, b])
        self.assertEqual(frozenset([a, b]), heads(self.pool, "p", facts)[self.entity])
        resolved = revision(self.pool, self.entity, "reviewer", "1",
                            {"state": "open", "reason": "need another control"}, [a, b])
        joined = merge(facts, [resolved])
        self.assertEqual(frozenset([resolved]), heads(self.pool, "p", joined)[self.entity])
        unseen = revision(self.pool, self.entity, "c", "1", {"state": "retired"}, [a])
        self.assertEqual(frozenset([resolved, unseen]),
                         heads(self.pool, "p", merge(joined, [unseen]))[self.entity])

    def test_origin_names_prevent_alias_collapse(self):
        a = revision(self.pool, "p/a/policy-000", "a", "1", {})
        b = revision(self.pool, "p/b/policy-000", "b", "1", {})
        self.assertEqual(2, len(heads(self.pool, "p", merge([a], [b]))))

    def test_operation_retry_is_idempotent_but_changed_bytes_conflict(self):
        same = revision(self.pool, self.entity, "w0", "start", {"state": "open"})
        self.assertEqual(self.base, same)
        changed = revision(self.pool, self.entity, "w0", "start", {"state": "done"})
        with self.assertRaises(IntegrityError):
            heads(self.pool, "p", merge(self.base_facts, [changed]))

    def test_different_project_cannot_be_silently_renamed(self):
        foreign = revision(self.pool, "other/a/note", "a", "1", {})
        with self.assertRaises(IntegrityError):
            heads(self.pool, "p", merge(self.base_facts, [foreign]))

    def test_causal_parent_must_exist_and_belong_to_entity(self):
        a = revision(self.pool, self.entity, "a", "1", {}, [digest(b"absent")])
        b = revision(self.pool, "p/b/different", "b", "1", {}, [self.base])
        for facts in [merge(self.base_facts, [a]), merge(self.base_facts, [b])]:
            with self.assertRaises(IntegrityError):
                heads(self.pool, "p", facts)

    def test_acyclic_branches_can_form_cycle_on_union(self):
        a, b = frozenset([("x", "y")]), frozenset([("y", "x"), ("z", "x")])
        self.assertFalse(cyclic_edges(a))
        self.assertFalse(cyclic_edges(b))
        self.assertEqual(frozenset([("x", "y"), ("y", "x")]), cyclic_edges(a | b))
        self.assertEqual(cyclic_edges(a | b), cyclic_edges(b | a))

    def branches(self):
        return [merge(self.base_facts, [revision(self.pool, f"p/r{i}/policy-000",
                    f"w{i+1}", "one", {"state": "open"})]) for i in range(20)]

    def test_twenty_way_union_converges_in_different_orders(self):
        branches = self.branches()
        expected = merge(*branches)
        self.assertEqual(21, len(expected))
        self.assertEqual(merge(*branches[:10], *branches[10:]), expected)
        self.assertEqual(merge(expected, expected), expected)
        self.assertEqual(merge(merge(*branches[:7]), merge(*branches[7:])), expected)
        rng = random.Random(450)
        for _ in range(20):
            rng.shuffle(branches)
            merged = frozenset()
            for branch in branches:
                merged = merge(branch, merged)
            self.assertEqual(expected, merged)
            self.assertEqual(21, len(heads(self.pool, "p", merged)))

    def test_twenty_stale_writers_rebase_without_lost_facts(self):
        ref = Ref(self.pool, "p")
        original = ref.publish(self.base_facts, None)
        old_token, stale = ref.token, 0
        branches = self.branches()
        for branch in branches:
            try:
                ref.publish(merge(self.base_facts, branch), old_token)
            except StaleHead:
                stale += 1
                ref.publish(merge(ref.facts, branch), ref.token)
        self.assertEqual(19, stale)
        self.assertEqual(merge(*branches), ref.facts)
        self.assertEqual([self.base], self.pool.value(original)["facts"])
        with self.assertRaises(IntegrityError):
            ref.publish(self.base_facts, ref.token)

    def test_incomplete_upload_does_not_advance_head(self):
        ref = Ref(self.pool, "p")
        original = ref.publish(self.base_facts, None)
        token = ref.token
        incomplete = merge(self.base_facts, [digest(b"incomplete upload")])
        with self.assertRaises(IntegrityError):
            ref.publish(incomplete, token)
        self.assertEqual((original, token, self.base_facts),
                         (ref.commit, ref.token, ref.facts))


if __name__ == "__main__":
    unittest.main(verbosity=2)
