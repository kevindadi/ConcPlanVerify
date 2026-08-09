"""Offline tests for modular ConcIR merging."""

from __future__ import annotations

import json
import unittest

from cir_workflow.merge import MergeError, load_module_bundle, merge_modules


def fragment(**overrides):
    base = {
        "program": "proj",
        "resources": [],
        "protection": [],
        "functions": [],
        "entry": "main",
    }
    base.update(overrides)
    return base


def module(name, concir):
    return {"module": name, "concir": concir}


class MergeTests(unittest.TestCase):
    def test_basic_merge_tags_functions_and_resolves_entry(self):
        modules = [
            module(
                "main",
                fragment(
                    resources=[{"name": "mtx", "kind": "sync", "type": "Mutex", "mode": "Sync"}],
                    functions=[
                        {
                            "name": "main",
                            "kind": "normal",
                            "body": [
                                {"sid": "s1", "op": ["spawn", "worker"], "transfer": ["next", "s2"]},
                                {"sid": "s2", "op": ["join", "worker"], "transfer": ["next", "s3"]},
                                {"sid": "s3", "op": "return", "transfer": "return"},
                            ],
                        }
                    ],
                    entry="main",
                ),
            ),
            module(
                "worker",
                fragment(
                    functions=[
                        {
                            "name": "worker",
                            "kind": "closure",
                            "body": [
                                {"sid": "s1", "op": ["res_op", "mtx", "lock"], "transfer": ["next", "s2"]},
                                {"sid": "s2", "op": ["res_op", "mtx", "drop"], "transfer": ["next", "s3"]},
                                {"sid": "s3", "op": "return", "transfer": "return"},
                            ],
                        }
                    ]
                ),
            ),
        ]

        merged, fn_to_module = merge_modules(modules, program_name="proj", entry_module="main")

        self.assertEqual(merged["entry"], "main")
        self.assertEqual([f["name"] for f in merged["functions"]], ["main", "worker"])
        self.assertEqual(fn_to_module, {"main": "main", "worker": "worker"})
        # Cross-module reference resolves after merge.
        self.assertEqual(
            {f["name"]: f.get("module") for f in merged["functions"]},
            {"main": "main", "worker": "worker"},
        )
        # Shared resource declared once.
        self.assertEqual(len(merged["resources"]), 1)
        self.assertEqual(merged["resources"][0]["name"], "mtx")

    def test_duplicate_function_name_is_an_error(self):
        body = [
            {"sid": "s1", "op": "return", "transfer": "return"},
        ]
        modules = [
            module("a", fragment(functions=[{"name": "dup", "kind": "normal", "body": body}])),
            module("b", fragment(functions=[{"name": "dup", "kind": "normal", "body": body}])),
        ]
        with self.assertRaises(MergeError) as ctx:
            merge_modules(modules, program_name="p", entry_module="a")
        self.assertIn("dup", str(ctx.exception))

    def test_inconsistent_resource_is_an_error(self):
        modules = [
            module(
                "a",
                fragment(resources=[{"name": "mtx", "kind": "sync", "type": "Mutex", "mode": "Sync"}]),
            ),
            module(
                "b",
                fragment(
                    resources=[
                        {
                            "name": "mtx",
                            "kind": "sync",
                            "type": "Mutex",
                            "mode": "Async",
                        }
                    ]
                ),
            ),
        ]
        with self.assertRaises(MergeError) as ctx:
            merge_modules(modules, program_name="p", entry_module="a")
        self.assertIn("inconsistently", str(ctx.exception))

    def test_consistent_shared_resource_is_accepted(self):
        resources = [{"name": "mtx", "kind": "sync", "type": "Mutex", "mode": "Sync"}]
        modules = [
            module("a", fragment(resources=resources)),
            module("b", fragment(resources=resources)),
        ]
        merged, _ = merge_modules(modules, program_name="p", entry_module="a")
        self.assertEqual(len(merged["resources"]), 1)

    def test_missing_entry_module_is_an_error(self):
        modules = [module("a", fragment())]
        with self.assertRaises(MergeError):
            merge_modules(modules, program_name="p", entry_module="nope")

    def test_entry_module_without_entry_is_an_error(self):
        modules = [module("a", fragment(functions=[], entry=None))]
        # entry is removed entirely for module "a"
        modules[0]["concir"] = fragment()
        modules[0]["concir"].pop("entry", None)
        with self.assertRaises(MergeError):
            merge_modules(modules, program_name="p", entry_module="a")

    def test_duplicate_goal_id_is_an_error(self):
        goal = {"id": "g1", "marking": {"main.ret": 1}}
        modules = [
            module("a", fragment(goals=[goal])),
            module("b", fragment(goals=[goal])),
        ]
        with self.assertRaises(MergeError) as ctx:
            merge_modules(modules, program_name="p", entry_module="a")
        self.assertIn("goal id", str(ctx.exception))

    def test_protection_deduplicated(self):
        prot = [{"var": "count", "lock": "mtx"}]
        modules = [
            module("a", fragment(protection=prot)),
            module("b", fragment(protection=prot)),
        ]
        merged, _ = merge_modules(modules, program_name="p", entry_module="a")
        self.assertEqual(len(merged["protection"]), 1)

    def test_load_module_bundle(self):
        bundle = {
            "program": "proj",
            "entry_module": "main",
            "modules": [module("main", fragment())],
        }
        modules, program_name, entry_module = load_module_bundle(bundle)
        self.assertEqual(program_name, "proj")
        self.assertEqual(entry_module, "main")
        self.assertEqual(len(modules), 1)

    def test_load_module_bundle_roundtrip_via_json(self):
        bundle = json.loads(
            json.dumps(
                {
                    "program": "proj",
                    "entry_module": "main",
                    "modules": [module("main", fragment())],
                }
            )
        )
        modules, program_name, entry_module = load_module_bundle(bundle)
        self.assertEqual(modules[0]["module"], "main")


if __name__ == "__main__":
    unittest.main()
