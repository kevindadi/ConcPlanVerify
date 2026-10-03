import unittest

from cir_workflow.json_utils import extract_json


class ExtractJsonTests(unittest.TestCase):
    def test_plain_object_with_trailing_text(self):
        self.assertEqual(extract_json('{"program": "demo"}\nDone.'), '{"program": "demo"}')

    def test_markdown_fence(self):
        self.assertEqual(
            extract_json("```json\n{\"program\": \"demo\"}\n```") ,
            '{"program": "demo"}',
        )

    def test_thinking_block(self):
        self.assertEqual(
            extract_json('<think>reasoning</think>\n{"program":"demo"}'),
            '{"program":"demo"}',
        )

    def test_invalid_text_is_preserved_for_repair_feedback(self):
        text = "I cannot produce JSON"
        self.assertEqual(extract_json(text), text)


if __name__ == "__main__":
    unittest.main()
