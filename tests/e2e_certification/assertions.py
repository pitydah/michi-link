"""Assertion helpers for E2E certification checks."""

import json


def _dotted_get(data, path):
    value = data
    for key in path.split("."):
        if isinstance(value, dict):
            value = value.get(key)
        else:
            return None
    return value


def check(name, status_code, expected_status, body, expected_json=None,
          not_expected_fields=None, no_body=False, assert_ranges=None):
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

    if no_body:
        text = getattr(body, "text", "") or ""
        if text.strip():
            return {
                "name": name,
                "status": "fail",
                "detail": f"Expected an empty body, got {len(text)} bytes",
            }

    if expected_json or not_expected_fields or assert_ranges:
        try:
            data = body.json() if hasattr(body, "json") else body
        except (json.JSONDecodeError, AttributeError, ValueError):
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

    if assert_ranges:
        for key, spec in assert_ranges.items():
            actual = _dotted_get(data, key)
            lo = spec.get("min")
            hi = spec.get("max")
            if actual is None or not isinstance(actual, (int, float)):
                return {
                    "name": name,
                    "status": "fail",
                    "detail": f"Expected {key} to be a number in [{lo}, {hi}], got {actual}",
                }
            if (lo is not None and actual < lo) or (hi is not None and actual > hi):
                return {
                    "name": name,
                    "status": "fail",
                    "detail": f"Expected {key} in [{lo}, {hi}], got {actual}",
                }
            detail_parts.append(f"{key}={actual} (in [{lo}, {hi}])")

    return {
        "name": name,
        "status": "pass",
        "detail": "; ".join(detail_parts),
    }
