const fs = require("fs");
const path = require("path");
const Ajv = require("ajv");
const addFormats = require("ajv-formats");

const ROOT = path.resolve(__dirname, "../..");
const SCHEMAS_DIR = path.join(ROOT, "schemas");
const EXAMPLES_DIR = path.join(ROOT, "examples");
const SCHEMA_BASE = "https://michi.link/schemas/";
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

// Exact mapping: example file name -> schema base name (without .schema.json).
// Every example MUST have a mapping; any unmapped example fails the suite.
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
  "pair-confirm-response.json": "pair-confirm-response",
  "pair-start.json": "pair-start",
  "pair-start-response.json": "pair-start-response",
  "playback-control-seek.json": "playback-control",
  "playback-control-volume.json": "playback-control",
  "playback-state.json": "playback-state",
  "qr-payload.json": "qr-pairing",
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

  const pairStartBase = loadJSON(path.join(EXAMPLES_DIR, "pair-start.json"));
  const pairConfirmBase = loadJSON(path.join(EXAMPLES_DIR, "pair-confirm.json"));
  const pairStartResponseBase = loadJSON(path.join(EXAMPLES_DIR, "pair-start-response.json"));
  const pairConfirmResponseBase = loadJSON(path.join(EXAMPLES_DIR, "pair-confirm-response.json"));

  const serverInfoMicroBase = {
    service: "michi-micro-server",
    name: "Test Server",
    version: "0.1.0",
    api_version: "v1",
    roles: ["music_server", "library_host", "playback_host"],
    features: { library: true, search: true },
    auth: { required: true, strategy: "SERVER_CODE", token_refresh: true },
  };

  const serverInfoPlayerBase = {
    service: "michi-music-player",
    name: "Test Player",
    version: "0.1.0",
    api_version: "v1",
    roles: ["desktop_player", "library_master", "sync_host"],
    features: { library: true },
    auth: { required: true, strategy: "PLAYER_PASSWORD", token_refresh: false },
  };

  const serverInfoMobileBase = {
    service: "michi-mobile",
    name: "Test Mobile",
    version: "0.1.0",
    api_version: "v1",
    roles: ["mobile_player", "remote_controller", "sync_client"],
    features: { library: true },
    auth: { required: true, strategy: "SERVER_CODE", token_refresh: true },
  };

  const serverInfoStreamBase = {
    service: "michi-stream-standard",
    name: "Test Stream",
    version: "0.1.0",
    api_version: "v1-lite",
    roles: ["audio_receiver"],
    features: { session: true, volume: true, heartbeat: true },
    auth: { required: true, strategy: "RECEIVER_BUTTON", token_refresh: false },
  };

  const receiverStandardBase = {
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

  const receiverHifiBase = {
    ...receiverStandardBase,
    service: "michi-stream-hifi",
    audio: { codecs: ["pcm_s16le", "pcm_s24le"], max_sample_rate: 192000, max_channels: 2 },
  };

  const announceBase = loadJSON(path.join(EXAMPLES_DIR, "michi-identity-announce-signed.json"));

  const identityBase = {
    identity_scheme: "ed25519-blake3-v1",
    michi_id: "QlGQosQszLQse057MCaw32IAHXv-I5klmAAsbivIays",
    public_key: "KJN5aOu4gWhA0clmvmwqprYcwYI013vDNPx1jf90CpQ",
  };

  const trackBase = {
    id: "track_9a8b7c6d",
    title: "Bohemian Rhapsody",
    artist: "Queen",
    album: "A Night at the Opera",
    duration_ms: 354000,
  };

  const V = (base) => ajv.getSchema(SCHEMA_BASE + base + ".schema.json");
  const rejects = (label, base, payload) => {
    const validate = V(base);
    const valid = validate(payload);
    check(label, !valid, valid ? null : validate);
  };

  // --- Pairing negatives (canonical snake_case contract) ---
  rejects("negative: pair-start rejects camelCase \"deviceId\"", "pair-start", {
    ...pairStartBase,
    deviceId: "dev-01",
  });
  rejects("negative: pair-start rejects retired \"pairing_id\"", "pair-start", {
    ...pairStartBase,
    pairing_id: "pair-01",
  });
  rejects("negative: pair-start rejects retired \"pairing_code\"", "pair-start", {
    ...pairStartBase,
    pairing_code: "abcd-1234",
  });
  rejects("negative: pair-start rejects challenge_signature of 43 chars", "pair-start", {
    ...pairStartBase,
    challenge_signature: "aB3dE5fG7hI9jK1lM3nO5pQ7rS9tU1vW3xY5zA7bC",
  });
  rejects("negative: pair-start rejects public_key of 86 chars", "pair-start", {
    ...pairStartBase,
    public_key: "aB3dE5fG7hI9jK1lM3nO5pQ7rS9tU1vW3xY5zA7bC9dE1fG3hI5jK7lM9nO1pQ3rS5tU7vW9xYzA1",
  });
  rejects("negative: pair-start rejects short challenge_nonce", "pair-start", {
    ...pairStartBase,
    challenge_nonce: "abc",
  });
  rejects("negative: pair-confirm rejects retired \"pin_proof\"", "pair-confirm", {
    ...pairConfirmBase,
    pin_proof: "aB3dE5fG7hI9jK1lM3nO5pQ7rS9tU1vW3xY5zA7bC",
  });
  rejects("negative: pair-confirm rejects camelCase \"signedChallenge\"", "pair-confirm", {
    ...pairConfirmBase,
    signedChallenge: "aB3dE5fG7hI9jK1lM3nO5pQ7rS9tU1vW3xY5zA7bC",
  });
  rejects("negative: pair-confirm rejects retired \"pairing_code\"", "pair-confirm", {
    ...pairConfirmBase,
    pairing_code: "abcd-1234",
  });
  rejects("negative: pair-confirm rejects 5-digit PIN", "pair-confirm", {
    ...pairConfirmBase,
    pin: "48239",
  });
  rejects("negative: pair-confirm rejects missing session_id", "pair-confirm", {
    ...pairConfirmBase,
    session_id: undefined,
  });
  rejects("negative: pair-start-response rejects missing server_public_key", "pair-start-response", {
    ...pairStartResponseBase,
    server_public_key: undefined,
  });
  rejects("negative: pair-confirm-response rejects missing token", "pair-confirm-response", {
    ...pairConfirmResponseBase,
    token: undefined,
  });
  rejects("negative: pair-confirm-response rejects missing expires_in", "pair-confirm-response", {
    ...pairConfirmResponseBase,
    expires_in: undefined,
  });

  // --- Wire format negatives (base64url, no + / =, exact lengths) ---
  rejects("negative: michi-identity rejects 64-char hex michi_id", "michi-identity", {
    ...identityBase,
    michi_id: "aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
  });
  rejects("negative: discovery-announce rejects public_key containing '+'", "discovery-announce", {
    ...announceBase,
    public_key: "KJN5aOu4gWhA0clmvmwqprYcwYI013vDNPx1jf9+CpQ",
  });
  rejects("negative: discovery-announce rejects signature containing '/'", "discovery-announce", {
    ...announceBase,
    signature: "-uA_huP-ihEN6MWq4QL2OK4tiPAk-FDmr_olMoLGDUUjWHHomPQTUSMkwI2yJIgT1JZXx5oCxhb6RIOzJ3e/CA",
  });
  rejects("negative: discovery-announce rejects nonce containing '='", "discovery-announce", {
    ...announceBase,
    nonce: "IgYedKwBmRm-r6bvLRATL4UjdXUNyb9W=",
  });

  // --- server-info per-service profile negatives ---
  rejects("negative: server-info player rejects strategy SERVER_CODE", "server-info", {
    ...serverInfoPlayerBase,
    auth: { ...serverInfoPlayerBase.auth, strategy: "SERVER_CODE" },
  });
  rejects("negative: server-info micro rejects strategy PLAYER_PASSWORD", "server-info", {
    ...serverInfoMicroBase,
    auth: { ...serverInfoMicroBase.auth, strategy: "PLAYER_PASSWORD" },
  });
  rejects("negative: server-info stream rejects token_refresh true", "server-info", {
    ...serverInfoStreamBase,
    auth: { ...serverInfoStreamBase.auth, token_refresh: true },
  });
  rejects("negative: server-info stream rejects api_version v1", "server-info", {
    ...serverInfoStreamBase,
    api_version: "v1",
  });
  rejects("negative: server-info mobile rejects api_version v1-lite", "server-info", {
    ...serverInfoMobileBase,
    api_version: "v1-lite",
  });
  rejects("negative: server-info mobile rejects role audio_receiver", "server-info", {
    ...serverInfoMobileBase,
    roles: ["audio_receiver"],
  });
  rejects("negative: server-info rejects partial identity group (michi_id only)", "server-info", {
    ...serverInfoMicroBase,
    michi_id: "QlGQosQszLQse057MCaw32IAHXv-I5klmAAsbivIays",
  });
  rejects("negative: server-info rejects extra michi_link_version", "server-info", {
    ...serverInfoMicroBase,
    michi_link_version: "1.0.0",
  });
  rejects("negative: server-info rejects unknown service \"michi-big-server\"", "server-info", {
    ...serverInfoMicroBase,
    service: "michi-big-server",
  });

  // --- receiver-info tier negatives ---
  rejects("negative: receiver-info standard rejects pcm_s24le", "receiver-info", {
    ...receiverStandardBase,
    audio: { ...receiverStandardBase.audio, codecs: ["pcm_s24le"] },
  });
  rejects("negative: receiver-info hifi rejects codec \"opus\"", "receiver-info", {
    ...receiverHifiBase,
    audio: { ...receiverHifiBase.audio, codecs: ["opus"] },
  });
  rejects("negative: receiver-info hifi rejects missing pcm_s16le", "receiver-info", {
    ...receiverHifiBase,
    audio: { ...receiverHifiBase.audio, codecs: ["pcm_s24le"] },
  });
  rejects("negative: receiver-info rejects max_sample_rate 384000", "receiver-info", {
    ...receiverStandardBase,
    audio: { ...receiverStandardBase.audio, max_sample_rate: 384000 },
  });
  rejects("negative: receiver-info rejects max_channels 8", "receiver-info", {
    ...receiverStandardBase,
    audio: { ...receiverStandardBase.audio, max_channels: 8 },
  });

  // --- track path-field negatives ---
  rejects("negative: track rejects \"path\"", "track", {
    ...trackBase,
    path: "/data/music/track.flac",
  });
  rejects("negative: track rejects \"file_path\"", "track", {
    ...trackBase,
    file_path: "/data/music/track.flac",
  });
  rejects("negative: track rejects \"absolute_path\"", "track", {
    ...trackBase,
    absolute_path: "/data/music/track.flac",
  });

  // --- Error code negatives ---
  rejects("negative: error rejects unknown code \"NOPE\"", "error", {
    error: { code: "NOPE", message: "nope" },
  });
  {
    const validate = V("error");
    const payload = { error: { code: "PAIRING_NOT_FOUND", message: "No pairing session." } };
    check("positive: error accepts PAIRING_NOT_FOUND (20-code enum)", validate(payload), validate);
  }

  // --- Regression coverage (pre-convergence checks) ---
  rejects("negative: server-info rejects deprecated service \"michi-player\"", "server-info", {
    ...serverInfoMicroBase,
    service: "michi-player",
  });
  rejects("negative: server-info rejects api_version \"1.0.0\"", "server-info", {
    ...serverInfoMicroBase,
    api_version: "1.0.0",
  });
  rejects("negative: server-info rejects auth.required false", "server-info", {
    ...serverInfoMicroBase,
    auth: { ...serverInfoMicroBase.auth, required: false },
  });
  rejects("negative: server-info rejects non-boolean feature flag", "server-info", {
    ...serverInfoMicroBase,
    features: { ...serverInfoMicroBase.features, library: "yes" },
  });
  rejects("negative: playback-control rejects legacy \"action\" field", "playback-control", {
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
  rejects("negative: receiver-info rejects codec \"opus\" on standard receiver", "receiver-info", {
    ...receiverStandardBase,
    audio: { ...receiverStandardBase.audio, codecs: ["opus"] },
  });
  rejects("negative: receiver-info rejects api_version \"v1\"", "receiver-info", {
    ...receiverStandardBase,
    api_version: "v1",
  });
  rejects("negative: michi-identity rejects public_key with padding '='", "michi-identity", {
    ...identityBase,
    public_key: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
  });
  rejects("negative: discovery-announce rejects too-short nonce", "discovery-announce", {
    ...announceBase,
    nonce: "short",
  });
  rejects("negative: server-info rejects role \"home_server\"", "server-info", {
    ...serverInfoMicroBase,
    roles: ["home_server"],
  });
  rejects("negative: server-info rejects audio_receiver role for michi-music-player", "server-info", {
    ...serverInfoPlayerBase,
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
