const fs = require("fs");
const path = require("path");
const Ajv = require("ajv");
const addFormats = require("ajv-formats");

const ROOT = path.resolve(__dirname, "../..");
const SCHEMAS_DIR = path.join(ROOT, "schemas");
const EXAMPLES_DIR = path.join(ROOT, "examples");
const PASS = "\u001b[32m\u2713\u001b[0m";
const FAIL = "\u001b[31m\u2717\u001b[0m";

function loadJSON(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function schemaFiles() {
  return fs.readdirSync(SCHEMAS_DIR).filter((f) => f.endsWith(".schema.json")).sort();
}

function exampleFiles() {
  return fs.readdirSync(EXAMPLES_DIR).filter((f) => f.endsWith(".json")).sort();
}

const SCHEMA_BASE = "https://michi.link/schemas/";

// Exact mapping: example file name -> schema base name (without .schema.json).
const EXAMPLE_MAP = {
  "audio-chain.json": "audio-chain",
  "commit-mapping-result.json": "import-commit-result",
  "continue-on-server-request.json": "continue-on-server",
  "discovery-announce.json": "discovery-announce",
  "e2e-failure-auth.json": "e2e-report",
  "e2e-mobile-player-pass.json": "e2e-report",
  "e2e-player-micro-import-pass.json": "e2e-report",
  "event-playback-state-changed.json": "event",
  "identity-document.json": "michi-identity",
  "import-commit-result.json": "import-commit-result",
  "import-preflight-request.json": "import-preflight",
  "import-preflight-response.json": "import-preflight-response",
  "michi-identity-announce-signed.json": "discovery-announce",
  "pair-confirm.json": "pair-confirm",
  "pair-start.json": "pair-start",
  "playback-control-seek.json": "playback-control",
  "playback-control-volume.json": "playback-control",
  "playback-state.json": "playback-state",
  "queue-bulk-request.json": "queue-bulk",
  "queue.json": "queue",
  "queue-transfer-request.json": "queue-transfer",
  "queue-transfer-response.json": "queue-transfer-response",
  "receiver-hifi-info.json": "receiver-info",
  "receiver-standard-info.json": "receiver-info",
  "server-info-micro-server.json": "server-info",
  "server-info-mobile.json": "server-info",
  "server-info-player.json": "server-info",
  "sync-delta.json": "sync-delta",
  "sync-manifest.json": "sync-manifest",
  "tracks-bulk-request.json": "track-bulk",
  "upload-result.json": "import-upload-result",
  "upload-result-unconfirmed.json": "import-upload-result",
};

function schemaBaseFor(exampleName) {
  if (exampleName.startsWith("error-")) return "error";
  return EXAMPLE_MAP[exampleName] || null;
}

function printErrors(validate) {
  for (const err of validate.errors || []) {
    console.log(`       ${err.instancePath || "/"} ${err.keyword} ${err.message}`);
  }
}

function main() {
  const ajv = new Ajv({ strict: true, allErrors: true });
  addFormats(ajv);

  let passed = 0;
  let failed = 0;

  function check(label, ok, validate) {
    if (ok) {
      console.log(`${PASS} ${label}`);
      passed++;
    } else {
      console.log(`${FAIL} ${label}`);
      if (validate) printErrors(validate);
      failed++;
    }
  }

  // Pass 1: register every schema by its $id so cross-schema $refs resolve.
  const schemaIds = {};
  for (const name of schemaFiles()) {
    const schema = loadJSON(path.join(SCHEMAS_DIR, name));
    const id = SCHEMA_BASE + name;
    schemaIds[name] = id;
    try {
      ajv.addSchema(schema);
    } catch (err) {
      console.error(`FATAL: failed to register schema ${name}: ${err.message}`);
      process.exit(1);
    }
  }

  // Pass 2: compile every schema. Each successful compilation counts as a check.
  for (const name of schemaFiles()) {
    let validate;
    try {
      validate = ajv.getSchema(schemaIds[name]);
    } catch (err) {
      validate = null;
    }
    check(`schema ${name} compiles`, typeof validate === "function", null);
  }

  // Validate every example against its mapped schema with real AJV.
  const orphaned = [];
  for (const name of exampleFiles()) {
    const base = schemaBaseFor(name);
    if (!base) {
      orphaned.push(name);
      continue;
    }
    const data = loadJSON(path.join(EXAMPLES_DIR, name));
    const validate = ajv.getSchema(SCHEMA_BASE + base + ".schema.json");
    if (typeof validate !== "function") {
      check(`example ${name} -> ${base}.schema.json compiled`, false, null);
      continue;
    }
    check(`example ${name} matches ${base}.schema.json`, validate(data), validate);
  }

  // Anti-orphan: any example without a schema mapping fails the suite.
  for (const name of orphaned) {
    check(`example ${name} has no schema mapping`, false, null);
  }

  // --- Negative checks: minimal valid base payloads, one mutation each ---

  const serverInfoBase = {
    service: "michi-micro-server",
    name: "Test Server",
    version: "0.1.0",
    api_version: "v1",
    roles: ["music_server", "library_host", "playback_host"],
    features: { library: true, search: true },
    auth: { required: true, strategy: "SERVER_CODE", token_refresh: true },
  };

  const receiverBase = {
    service: "michi-stream-standard",
    name: "Test Receiver",
    version: "0.1.0",
    firmware: "0.1.0",
    api_version: "v1-lite",
    roles: ["audio_receiver"],
    auth: { required: true, strategy: "RECEIVER_BUTTON", token_refresh: false },
    audio: { codecs: ["pcm_s16le"], max_sample_rate: 96000, max_channels: 2 },
    features: { session: true, volume: true, heartbeat: true },
  };

  const announceBase = {
    device_id: "dev-01",
    name: "Test Device",
    service: "michi-micro-server",
    roles: ["music_server"],
    api_version: "v1",
    host: "192.168.1.10",
    port: 8400,
    features: { library: true },
    michi_id: "QlGQosQszLQse057MCaw32IAHXv-I5klmAAsbivIays",
    public_key: "KJN5aOu4gWhA0clmvmwqprYcwYI013vDNPx1jf90CpQ",
    signature: "-uA_huP-ihEN6MWq4QL2OK4tiPAk-FDmr_olMoLGDUUjWHHomPQTUSMkwI2yJIgT1JZXx5oCxhb6RIOzJ3eTCA",
    timestamp_ms: Date.now(),
    nonce: "IgYedKwBmRm-r6bvLRATL4UjdXUNyb9W",
  };

  const identityBase = {
    identity_scheme: "ed25519-blake3-v1",
    michi_id: "QlGQosQszLQse057MCaw32IAHXv-I5klmAAsbivIays",
    public_key: "KJN5aOu4gWhA0clmvmwqprYcwYI013vDNPx1jf90CpQ",
  };

  const V = (base) => ajv.getSchema(SCHEMA_BASE + base + ".schema.json");
  const rejects = (label, base, payload) => {
    const validate = V(base);
    const valid = validate(payload);
    check(label, !valid, valid ? null : validate);
  };

  rejects("negative: server-info rejects service \"michi-big-server\"", "server-info", {
    ...serverInfoBase,
    service: "michi-big-server",
  });
  rejects("negative: server-info rejects deprecated service \"michi-player\"", "server-info", {
    ...serverInfoBase,
    service: "michi-player",
  });
  rejects("negative: server-info rejects extra michi_link_version", "server-info", {
    ...serverInfoBase,
    michi_link_version: "1.0.0",
  });
  rejects("negative: server-info rejects api_version \"1.0.0\"", "server-info", {
    ...serverInfoBase,
    api_version: "1.0.0",
  });
  rejects("negative: server-info rejects auth.required false", "server-info", {
    ...serverInfoBase,
    auth: { ...serverInfoBase.auth, required: false },
  });
  rejects("negative: server-info rejects non-boolean feature", "server-info", {
    ...serverInfoBase,
    features: { ...serverInfoBase.features, library: "yes" },
  });
  rejects("negative: playback-control rejects \"action\" field", "playback-control", {
    action: "play",
  });
  rejects("negative: discovery-announce rejects signed announce missing nonce", "discovery-announce", {
    ...announceBase,
    nonce: undefined,
  });
  rejects("negative: discovery-announce rejects signature without timestamp_ms", "discovery-announce", {
    ...announceBase,
    timestamp_ms: undefined,
  });
  rejects("negative: receiver-info rejects codec \"opus\"", "receiver-info", {
    ...receiverBase,
    audio: { ...receiverBase.audio, codecs: ["opus"] },
  });
  rejects("negative: receiver-info rejects api_version \"v1\"", "receiver-info", {
    ...receiverBase,
    api_version: "v1",
  });
  rejects("negative: michi-identity rejects non-base64url public_key", "michi-identity", {
    ...identityBase,
    public_key: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
  });
  rejects("negative: michi-identity rejects 64-char hex michi_id", "michi-identity", {
    ...identityBase,
    michi_id: "aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
  });
  rejects("negative: discovery-announce rejects too-short nonce", "discovery-announce", {
    ...announceBase,
    nonce: "short",
  });
  rejects("negative: pair-confirm rejects pairing_code violating pattern", "pair-confirm", {
    device_id: "d3c4e5f6a7b89012cdef123456789012",
    pairing_code: "abcd-1234",
  });
  rejects("negative: server-info rejects role \"home_server\"", "server-info", {
    ...serverInfoBase,
    roles: ["home_server"],
  });
  rejects("negative: error rejects code \"NOPE\"", "error", {
    error: { code: "NOPE", message: "nope" },
  });
  rejects("negative: server-info if/then rejects audio_receiver role for michi-music-player", "server-info", {
    ...serverInfoBase,
    service: "michi-music-player",
    roles: ["audio_receiver"],
  });
  rejects("negative: discovery-announce rejects null features", "discovery-announce", {
    ...announceBase,
    features: null,
  });
  rejects("negative: event rejects type \"queue.updated\"", "event", {
    type: "queue.updated",
    data: {},
    timestamp: "2026-08-05T12:00:00Z",
  });
  rejects("negative: server-info rejects missing auth", "server-info", {
    service: "michi-micro-server",
    name: "Test Server",
    version: "0.1.0",
    api_version: "v1",
    roles: ["music_server"],
    features: { library: true },
  });
  rejects("negative: error rejects missing message", "error", {
    error: { code: "NOT_FOUND" },
  });

  const total = passed + failed;
  console.log(`\n${total} checks: ${passed} passed, ${failed} failed`);
  process.exit(failed > 0 ? 1 : 0);
}

main();
