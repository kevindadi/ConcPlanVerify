import os
import tempfile
import unittest
from pathlib import Path

from cir_workflow.env import load_dotenv


class DotenvTests(unittest.TestCase):
    def test_does_not_override_existing_environment(self):
        key = "CIR_WORKFLOW_TEST_KEY"
        old = os.environ.get(key)
        os.environ[key] = "existing"
        try:
            with tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / ".env"
                path.write_text(f'{key}="from-file"\nNEW_CIR_WORKFLOW_KEY=loaded\n')
                load_dotenv(path)
            self.assertEqual(os.environ[key], "existing")
            self.assertEqual(os.environ["NEW_CIR_WORKFLOW_KEY"], "loaded")
        finally:
            if old is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = old
            os.environ.pop("NEW_CIR_WORKFLOW_KEY", None)


if __name__ == "__main__":
    unittest.main()
