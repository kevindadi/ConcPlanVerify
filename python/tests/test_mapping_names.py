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
    def test_short_names_bind_and_single_channel_endpoints_share_it(self):
        resources = [
            {"name": "a_mutex0", "kind": "Mutex"},
            {"name": "b_mutex0", "kind": "Mutex"},
            {"name": "tx", "kind": "Channel"},
            {"name": "rx", "kind": "Channel"},
            {"name": "w1", "kind": "Spawn"},
        ]
        mapping, prov = mapping_to_cir(resources, CIR)
        self.assertEqual(mapping["a_mutex0"], "main::a")
        self.assertEqual(mapping["b_mutex0"], "main::b")
        self.assertEqual(mapping["tx"], "main::ch")
        self.assertEqual(mapping["rx"], "main::ch")
        self.assertEqual(mapping["w1"], "main::w1")
        self.assertEqual(prov["ambiguous"], [])

    def test_unknown_mutex_is_ambiguous_not_zipped_by_order(self):
        resources = [{"name": "other_mutex0", "kind": "Mutex"}]
        mapping, prov = mapping_to_cir(resources, CIR)
        self.assertNotIn("other_mutex0", mapping)
        self.assertEqual(prov["ambiguous"][0]["rust"], "other_mutex0")


if __name__ == "__main__":
    unittest.main()
