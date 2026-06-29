"""Assertion helpers for E2E certification checks."""

import json


def check(name, status_code, expected_status, body, expected_json=None, not_expected_fields=None):
    """
    Run a single assertion against an HTTP response.

    Returns a dict with 'name', 'status' ('pass'|'fail'), and 'detail'.
    """
    if status_code != expected_status:
        return {
            "name": name,
            "status": "fail",
            "detail": f"Expected status {expected_status}, got {status_code}",
        }

    detail_parts = [f"HTTP {status_code}"]

    if expected_json or not_expected_fields:
        try:
            data = body.json() if hasattr(body, "json") else body
        except (json.JSONDecodeError, AttributeError):
            if expected_json:
                return {
                    "name": name,
                    "status": "fail",
                    "detail": f"Expected JSON body but got non-JSON response",
                }
            data = {}

    if expected_json:
        for key, expected_value in expected_json.items():
            actual = data
            for k in key.split("."):
                if isinstance(actual, dict):
                    actual = actual.get(k)
                else:
                    actual = None
                    break
            if actual is None or actual != expected_value:
                return {
                    "name": name,
                    "status": "fail",
                    "detail": f"Expected {key}={expected_value}, got {actual}",
                }
            detail_parts.append(f"{key}={actual}")

    if not_expected_fields:
        for field in not_expected_fields:
            actual = data
            for k in field.split("."):
                if isinstance(actual, dict):
                    actual = actual.get(k)
                else:
                    actual = None
                    break
            if actual is not None:
                return {
                    "name": name,
                    "status": "fail",
                    "detail": f"Field '{field}' should not be present but found: {actual}",
                }

    return {
        "name": name,
        "status": "pass",
        "detail": "; ".join(detail_parts),
    }
