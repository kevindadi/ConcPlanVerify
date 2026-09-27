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
            {"name": "w1", "kind": "Spawn"},
        ]
        mapping, prov = mapping_to_cir(resources, CIR)
        self.assertEqual(mapping["a_mutex0"], "main::a")
        self.assertEqual(mapping["b_mutex0"], "main::b")
        self.assertEqual(mapping["ch_tx"], "main::ch")
        self.assertEqual(mapping["ch_rx"], "main::ch")
        self.assertEqual(mapping["w1"], "main::w1")
        self.assertEqual(prov["ambiguous"], [])

    def test_unknown_mutex_is_ambiguous_not_zipped_by_order(self):
        resources = [{"name": "other_mutex0", "kind": "Mutex"}]
        mapping, prov = mapping_to_cir(resources, CIR)
        self.assertNotIn("other_mutex0", mapping)
        self.assertEqual(prov["ambiguous"][0]["rust"], "other_mutex0")

    def test_elimination_is_a_suggestion_not_a_binding(self):
        # CIR has two mutexes; Rust names one exactly and one unknown. The
        # unknown must NOT be bound to the remaining CIR mutex by elimination.
        resources = [{"name": "a", "kind": "Mutex"}, {"name": "other_mutex0", "kind": "Mutex"}]
        mapping, prov = mapping_to_cir(resources, CIR)
        self.assertEqual(mapping["a"], "main::a")
        self.assertNotIn("other_mutex0", mapping)
        amb = {x["rust"]: x for x in prov["ambiguous"]}
        self.assertEqual(amb["other_mutex0"]["suggestion"], "main::b")
        self.assertEqual(amb["other_mutex0"]["candidates"], ["main::b"])

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
