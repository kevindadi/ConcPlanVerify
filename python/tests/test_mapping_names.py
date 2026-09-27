import unittest

from cir_workflow.generation import mapping_to_cir

CIR = {
    "modules": [{
        "name": "main",
        "resources": [
            {"name": "a", "type": "Mutex"},
            {"name": "b", "type": "Mutex"},
            {"name": "ch", "type": "Channel"},
        ],
        "functions": [
            {"name": "main"},
            {"name": "w1"},
            {"name": "w2"},
        ],
    }]
}


class MappingTests(unittest.TestCase):
    def test_named_endpoints_bind_by_channel_token(self):
        resources = [
            {"name": "a_mutex0", "kind": "Mutex"},
            {"name": "b_mutex0", "kind": "Mutex"},
            {"name": "ch_tx", "kind": "Channel"},
            {"name": "ch_rx", "kind": "Channel"},
            {"name": "spawn1", "kind": "Spawn", "entry": "w1", "unique_entry": True},
        ]
        mapping, prov = mapping_to_cir(resources, CIR)
        self.assertEqual(mapping["a_mutex0"], "main::a")
        self.assertEqual(mapping["b_mutex0"], "main::b")
        self.assertEqual(mapping["ch_tx"], "main::ch")
        self.assertEqual(mapping["ch_rx"], "main::ch")
        self.assertEqual(mapping["spawn1"], "main::w1")
        self.assertEqual(prov["ambiguous"], [])

    def test_module_prefix_is_not_a_binding(self):
        # CIR has only main::a; `main_other_mutex0` must NOT be proven to be it.
        cir = {"modules": [{"name": "main", "resources": [{"name": "a", "type": "Mutex"}],
                            "functions": [{"name": "main"}]}]}
        mapping, prov = mapping_to_cir([{"name": "main_other_mutex0", "kind": "Mutex"}], cir)
        self.assertNotIn("main_other_mutex0", mapping)
        self.assertEqual(prov["ambiguous"][0]["rust"], "main_other_mutex0")

    def test_spawn_handle_substring_is_not_a_binding(self):
        # `not_t1_worker` must not be proven to be thread t1 by substring.
        cir = {"modules": [{"name": "main", "resources": [],
                            "functions": [{"name": "main"}, {"name": "t1"}]}]}
        mapping, prov = mapping_to_cir(
            [{"name": "not_t1_worker", "kind": "Spawn"}], cir)
        self.assertNotIn("not_t1_worker", mapping)
        self.assertTrue(prov["ambiguous"])

    def test_two_instances_same_name_are_a_collision(self):
        resources = [{"name": "m", "kind": "Mutex", "site": "94"},
                     {"name": "m", "kind": "Mutex", "site": "130"}]
        cir = {"modules": [{"name": "main", "resources": [{"name": "m", "type": "Mutex"}],
                            "functions": [{"name": "main"}]}]}
        mapping, prov = mapping_to_cir(resources, cir)
        self.assertNotIn("m", mapping)  # cannot be two identities at once
        self.assertTrue(any("duplicate" in a.get("reason", "")
                            or "collision" in a.get("reason", "")
                            for a in prov["ambiguous"]))

    def test_ambiguous_closure_entry_is_unresolved(self):
        resources = [{"name": "helper", "kind": "Spawn", "entry": "helper",
                      "unique_entry": False}]
        mapping, prov = mapping_to_cir(resources, CIR)
        self.assertNotIn("helper", mapping)
        self.assertTrue(prov["ambiguous"])

    def test_elimination_is_a_suggestion_not_a_binding(self):
        resources = [{"name": "a", "kind": "Mutex"}, {"name": "other_mutex0", "kind": "Mutex"}]
        mapping, prov = mapping_to_cir(resources, CIR)
        self.assertEqual(mapping["a"], "main::a")
        self.assertNotIn("other_mutex0", mapping)
        amb = {x["rust"]: x for x in prov["ambiguous"]}
        self.assertEqual(amb["other_mutex0"]["suggestion"], "main::b")

    def test_generic_endpoints_with_two_channels_are_ambiguous(self):
        resources = [{"name": "tx", "kind": "Channel"}, {"name": "rx", "kind": "Channel"}]
        cir2 = {"modules": [{"name": "main", "resources": [
            {"name": "ch1", "type": "Channel"}, {"name": "ch2", "type": "Channel"}],
            "functions": [{"name": "main"}]}]}
        mapping, prov = mapping_to_cir(resources, cir2)
        self.assertNotIn("tx", mapping)
        self.assertNotIn("rx", mapping)
        self.assertTrue(prov["ambiguous"])


if __name__ == "__main__":
    unittest.main()
