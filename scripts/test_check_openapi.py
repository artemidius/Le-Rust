from __future__ import annotations

import hashlib
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest

import yaml


SCRIPT = pathlib.Path(__file__).with_name("check-openapi.py")


def source_document() -> dict:
    return {
        "openapi": "3.1.0",
        "paths": {
            "/v1/models": {
                "get": {
                    "responses": {
                        "200": {
                            "content": {
                                "application/json": {
                                    "schema": {"$ref": "#/components/schemas/ModelList"}
                                }
                            }
                        }
                    }
                }
            },
            "/v1/models/{model_id}": {
                "get": {
                    "parameters": [
                        {
                            "name": "model_id",
                            "in": "path",
                            "required": True,
                            "schema": {"type": "string"},
                        }
                    ],
                    "responses": {"200": {"description": "ok"}},
                }
            },
            "/v1/chat/completions": {
                "post": {
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/ChatCompletionRequest"
                                }
                            }
                        }
                    },
                    "responses": {"200": {"description": "ok"}},
                }
            },
            "/v1/fim/completions": {
                "post": {
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "schema": {"$ref": "#/components/schemas/FimRequest"}
                            }
                        }
                    },
                    "responses": {"200": {"description": "ok"}},
                }
            },
        },
        "components": {
            "schemas": {
                "ModelList": {
                    "type": "object",
                    "properties": {
                        "data": {
                            "type": "array",
                            "items": {"$ref": "#/components/schemas/ModelCard"},
                        }
                    },
                },
                "ModelCard": {
                    "type": "object",
                    "required": ["id"],
                    "properties": {"id": {"type": "string"}},
                },
                "ChatCompletionRequest": {
                    "type": "object",
                    "required": ["model", "messages"],
                    "properties": {
                        "model": {"type": "string"},
                        "messages": {"type": "array"},
                    },
                },
                "FimRequest": {
                    "type": "object",
                    "required": ["model", "prompt"],
                    "properties": {
                        "model": {"type": "string"},
                        "prompt": {"type": "string"},
                    },
                },
            }
        },
    }


class DriftClassificationTest(unittest.TestCase):
    def test_exact_unrelated_and_covered_changes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            candidate = root / "openapi.yaml"
            manifest = root / "source.json"
            coverage = root / "coverage.toml"
            snapshot = root / "covered.json"
            report = root / "report.json"

            baseline = yaml.safe_dump(source_document(), sort_keys=True).encode()
            candidate.write_bytes(baseline)
            manifest.write_text(
                json.dumps(
                    {
                        "source": "https://invalid.example/openapi.yaml",
                        "sha256": hashlib.sha256(baseline).hexdigest(),
                    }
                )
            )
            coverage.write_text(
                """
[[operation]]
method = "GET"
path = "/v1/models"
public_method = "Models::list"

[[operation]]
method = "GET"
path = "/v1/models/{model_id}"
public_method = "Models::retrieve"

[[operation]]
method = "POST"
path = "/v1/chat/completions"
public_method = "Chat::complete"

[[operation]]
method = "POST"
path = "/v1/chat/completions"
public_method = "Chat::stream"

[[operation]]
method = "POST"
path = "/v1/fim/completions"
public_method = "Fim::complete"
"""
            )

            common = [
                sys.executable,
                str(SCRIPT),
                "--manifest",
                str(manifest),
                "--coverage",
                str(coverage),
                "--snapshot",
                str(snapshot),
                "--candidate",
                str(candidate),
                "--report",
                str(report),
            ]
            written = subprocess.run(
                [*common, "--write-snapshot"], capture_output=True, text=True
            )
            self.assertEqual(written.returncode, 0, written.stderr)

            exact = subprocess.run(common, capture_output=True, text=True)
            self.assertEqual(exact.returncode, 0, exact.stderr)
            self.assertEqual(json.loads(report.read_text())["status"], "exact_match")

            unrelated_document = source_document()
            unrelated_document["paths"]["/v1/unrelated"] = {
                "get": {"responses": {"200": {"description": "ok"}}}
            }
            candidate.write_text(yaml.safe_dump(unrelated_document, sort_keys=True))
            unrelated = subprocess.run(common, capture_output=True, text=True)
            self.assertEqual(unrelated.returncode, 0, unrelated.stderr)
            self.assertEqual(
                json.loads(report.read_text())["status"], "unrelated_change"
            )

            covered_document = source_document()
            covered_document["components"]["schemas"]["ChatCompletionRequest"][
                "properties"
            ]["new_required_behavior"] = {"type": "boolean"}
            candidate.write_text(yaml.safe_dump(covered_document, sort_keys=True))
            covered = subprocess.run(common, capture_output=True, text=True)
            self.assertEqual(covered.returncode, 1, covered.stderr)
            payload = json.loads(report.read_text())
            self.assertEqual(payload["status"], "covered_change")
            self.assertEqual(len(payload["affected_operations"]), 1)
            self.assertEqual(
                payload["affected_operations"][0]["path"],
                "/v1/chat/completions",
            )
            self.assertEqual(
                payload["affected_operations"][0]["public_methods"],
                ["Chat::complete", "Chat::stream"],
            )


if __name__ == "__main__":
    unittest.main()
