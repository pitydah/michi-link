"""Report builder for E2E certification results."""

import json
import os
from datetime import datetime, timezone


class ReportBuilder:
    """Builds and saves E2E certification reports in the standard format."""

    def __init__(self, scenario_id, scenario_name, server_type, client_type, certifier):
        self.scenario_id = scenario_id
        self.scenario_name = scenario_name
        self.server_type = server_type
        self.client_type = client_type
        self.certifier = certifier
        self.checks = []
        self.errors = []
        self.timestamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    def add_check(self, name, status, detail=""):
        self.checks.append({
            "name": name,
            "status": status,
            "detail": detail,
        })
        if status == "fail":
            self.errors.append(f"{name}: {detail}")

    @property
    def status(self):
        if not self.checks:
            return "fail"
        if any(c["status"] == "fail" for c in self.checks):
            return "fail"
        if any(c["status"] == "partial" for c in self.checks):
            return "partial"
        return "pass"

    def build(self):
        return {
            "scenario": self.scenario_id,
            "name": self.scenario_name,
            "status": self.status,
            "server": {"type": self.server_type, "version": "0.1.0"},
            "client": {"type": self.client_type, "version": "0.1.0"},
            "timestamp": self.timestamp,
            "checks": self.checks,
            "errors": self.errors,
            "certified_by": self.certifier,
            "certified_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        }

    def save(self, reports_dir):
        os.makedirs(reports_dir, exist_ok=True)
        date_str = datetime.now(timezone.utc).strftime("%Y%m%d")
        filename = f"{self.scenario_name}-{date_str}.json"
        filepath = os.path.join(reports_dir, filename)
        report = self.build()
        with open(filepath, "w") as f:
            json.dump(report, f, indent=2)
        return filepath
