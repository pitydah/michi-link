"""Vendored receiver-v1-lite JSON schema validation for E2E responses.

Loads every schema of contracts/receiver-v1-lite/schemas with a shared
$ref registry (jsonschema Draft 7, referencing) so scenario responses can
be validated against the canonical bundle.
"""

import json
import os
from pathlib import Path

from jsonschema import Draft7Validator
from referencing import Registry, Resource

BUNDLE_DIR = Path(__file__).resolve().parent.parent.parent / "contracts" / "receiver-v1-lite"
SCHEMAS_DIR = BUNDLE_DIR / "schemas"


class ContractSchemas:
    def __init__(self, schemas_dir=SCHEMAS_DIR):
        self.docs = {}
        for name in sorted(os.listdir(schemas_dir)):
            if name.endswith(".schema.json"):
                with open(schemas_dir / name, encoding="utf-8") as handle:
                    self.docs[name] = json.load(handle)
        self.registry = Registry().with_resources(
            (doc["$id"], Resource.from_contents(doc)) for doc in self.docs.values()
        )

    def validate(self, schema_name, body):
        """Returns a list of human-readable validation errors (empty = valid)."""
        if schema_name not in self.docs:
            return [f"unknown schema '{schema_name}' (not in the vendored bundle)"]
        validator = Draft7Validator(self.docs[schema_name], registry=self.registry)
        return [error.message for error in validator.iter_errors(body)]
