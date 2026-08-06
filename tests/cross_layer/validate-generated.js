const fs = require("fs");
const path = require("path");
const Ajv = require("ajv");
const addFormats = require("ajv-formats");

const ROOT = path.resolve(__dirname, "../..");
const SCHEMAS_DIR = path.join(ROOT, "schemas");
const VECTORS_DIR = path.join(ROOT, "tests", "vectors", "generated");
const SCHEMA_BASE = "https://michi.link/schemas/";
const PASS = "\u001b[32m\u2713\u001b[0m";
const FAIL = "\u001b[31m\u2717\u001b[0m";
const REGEN_HINT = "regenerate vectors with: cargo run --example generate_contract_vectors";

function loadJSON(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

// Vector base name -> schema base name (without .schema.json).
const VECTOR_SCHEMA_MAP = {
  "announce-micro": "discovery-announce",
  "announce-player": "discovery-announce",
  "announce-stream": "discovery-announce",
  "identity-document": "michi-identity",
  "pair-confirm-request": "pair-confirm",
  "pair-confirm-response": "pair-confirm-response",
  "pair-start-request": "pair-start",
  "pair-start-response": "pair-start-response",
  "qr-payload": "qr-pairing",
};

// Wire fields per vector: 43/86 are exact lengths, 22 is a minimum.
const WIRE_FIELDS = {
  "identity-document": { michi_id: 43, public_key: 43 },
  "announce-micro": { michi_id: 43, public_key: 43, signature: 86, nonce: 22 },
  "announce-player": { michi_id: 43, public_key: 43, signature: 86, nonce: 22 },
  "announce-stream": { michi_id: 43, public_key: 43, signature: 86, nonce: 22 },
  "pair-start-request": { michi_id: 43, public_key: 43, challenge_nonce: 22, challenge_signature: 86 },
  "pair-start-response": { server_michi_id: 43, server_public_key: 43 },
  "pair-confirm-request": { michi_id: 43, public_key: 43 },
  "pair-confirm-response": {},
  "qr-payload": { server_michi_id: 43, server_public_key: 43 },
};

const BASE64URL_NO_PADDING = /^[A-Za-z0-9_-]+$/;
const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const BANNED_PATH_FIELDS = ["path", "file_path", "absolute_path", "local_path", "mount_path"];

function printErrors(validate) {
  for (const err of validate.errors || []) {
    console.log(`       ${err.instancePath || "/"} ${err.keyword} ${err.message}`);
  }
}

function collectKeys(obj, acc) {
  for (const key of Object.keys(obj)) {
    acc.push(key);
    const value = obj[key];
    if (value !== null && typeof value === "object") collectKeys(value, acc);
  }
  return acc;
}

function allBooleanValues(obj) {
  return Object.values(obj).every((v) => typeof v === "boolean");
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

  const vectorBases = Object.keys(VECTOR_SCHEMA_MAP).sort();

  // 1. Existence checks: all 9 vectors must be present.
  const vectors = {};
  const missing = [];
  for (const base of vectorBases) {
    const filePath = path.join(VECTORS_DIR, `${base}.generated.json`);
    if (fs.existsSync(filePath)) {
      vectors[base] = loadJSON(filePath);
      check(`vector ${base}.generated.json exists`, true, null);
    } else {
      missing.push(base);
      check(`vector ${base}.generated.json exists (${REGEN_HINT})`, false, null);
    }
  }

  // 2. Schema validation: register all 41 schemas, validate each vector.
  for (const name of fs.readdirSync(SCHEMAS_DIR).filter((f) => f.endsWith(".schema.json")).sort()) {
    ajv.addSchema(loadJSON(path.join(SCHEMAS_DIR, name)));
  }

  for (const base of vectorBases) {
    if (!vectors[base]) {
      check(`vector ${base}.generated.json matches ${VECTOR_SCHEMA_MAP[base]}.schema.json (${REGEN_HINT})`, false, null);
      continue;
    }
    const validate = ajv.getSchema(SCHEMA_BASE + VECTOR_SCHEMA_MAP[base] + ".schema.json");
    const ok = typeof validate === "function" && validate(vectors[base]);
    check(`vector ${base}.generated.json matches ${VECTOR_SCHEMA_MAP[base]}.schema.json`, ok, typeof validate === "function" ? validate : null);
  }

  // 3. Wire checks: exact lengths, base64url charset, no '+', '/' or '='.
  for (const base of vectorBases) {
    if (!vectors[base]) continue;
    for (const [field, length] of Object.entries(WIRE_FIELDS[base])) {
      const value = vectors[base][field];
      const exact = length !== 22;
      const valid =
        typeof value === "string" &&
        BASE64URL_NO_PADDING.test(value) &&
        !/[+/=]/.test(value) &&
        (exact ? value.length === length : value.length >= length);
      check(
        `wire: ${base}.generated.json ${field} is ${exact ? "exactly" : "at least"} ${length} base64url chars without + / =`,
        valid,
        null
      );
    }
  }

  // 4. Cross-file coherence: the generator uses one server identity across
  //    identity-document and all announce vectors.
  const identityDoc = vectors["identity-document"];
  if (identityDoc) {
    for (const base of ["announce-player", "announce-micro", "announce-stream"]) {
      if (!vectors[base]) continue;
      check(
        `coherence: ${base}.generated.json michi_id matches identity-document.generated.json`,
        vectors[base].michi_id === identityDoc.michi_id,
        null
      );
      check(
        `coherence: ${base}.generated.json public_key matches identity-document.generated.json`,
        vectors[base].public_key === identityDoc.public_key,
        null
      );
    }
  }

  // 5. Announce profile checks.
  const PLAYER_ROLES = new Set(["desktop_player", "library_master", "sync_host"]);
  const MICRO_ROLES = new Set(["music_server", "library_host", "playback_host"]);
  if (vectors["announce-player"]) {
    const a = vectors["announce-player"];
    check(
      "announce: player is michi-music-player with api_version v1 and valid roles",
      a.service === "michi-music-player" &&
        a.api_version === "v1" &&
        Array.isArray(a.roles) &&
        a.roles.length > 0 &&
        a.roles.every((r) => PLAYER_ROLES.has(r)),
      null
    );
  }
  if (vectors["announce-micro"]) {
    const a = vectors["announce-micro"];
    check(
      "announce: micro is michi-micro-server with api_version v1 and valid roles",
      a.service === "michi-micro-server" &&
        a.api_version === "v1" &&
        Array.isArray(a.roles) &&
        a.roles.length > 0 &&
        a.roles.every((r) => MICRO_ROLES.has(r)),
      null
    );
  }
  if (vectors["announce-stream"]) {
    const a = vectors["announce-stream"];
    check(
      "announce: stream is michi-stream-hifi with api_version v1-lite and roles [audio_receiver]",
      a.service === "michi-stream-hifi" &&
        a.api_version === "v1-lite" &&
        Array.isArray(a.roles) &&
        a.roles.length === 1 &&
        a.roles[0] === "audio_receiver",
      null
    );
  }

  // 5b. Features must be boolean-only in every announce vector.
  for (const base of ["announce-player", "announce-micro", "announce-stream"]) {
    if (!vectors[base]) continue;
    const f = vectors[base].features;
    check(
      `announce: ${base}.generated.json features are all booleans`,
      f !== null && typeof f === "object" && allBooleanValues(f),
      null
    );
  }

  // 5c. No path-like fields anywhere in any generated vector (recursive).
  const allKeys = [];
  for (const base of vectorBases) {
    if (vectors[base]) collectKeys(vectors[base], allKeys);
  }
  for (const banned of BANNED_PATH_FIELDS) {
    check(
      `banned: no "${banned}" field in any generated vector (recursive)`,
      !allKeys.includes(banned),
      null
    );
  }

  // 6. Pairing vectors: exact key sets.
  const PAIR_START_KEYS = ["device_name", "device_type", "roles", "auth_strategy", "michi_id", "public_key", "challenge_nonce", "challenge_signature"];
  const PAIR_START_RESPONSE_KEYS = ["session_id", "expires_at", "attempts_remaining", "server_michi_id", "server_public_key"];
  const PAIR_CONFIRM_KEYS = ["session_id", "pin", "michi_id", "public_key"];
  const PAIR_CONFIRM_RESPONSE_KEYS = ["token", "refresh_token", "expires_in", "device_id", "server_id"];
  const keySetEq = (obj, keys) => {
    const actual = Object.keys(obj).sort();
    const expected = [...keys].sort();
    return actual.length === expected.length && actual.every((k, i) => k === expected[i]);
  };

  if (vectors["pair-start-request"]) {
    check(
      "pairing: pair-start-request has exactly the 8 canonical fields",
      keySetEq(vectors["pair-start-request"], PAIR_START_KEYS),
      null
    );
  }
  if (vectors["pair-start-response"]) {
    check(
      "pairing: pair-start-response has exactly the 5 canonical fields",
      keySetEq(vectors["pair-start-response"], PAIR_START_RESPONSE_KEYS),
      null
    );
  }
  if (vectors["pair-confirm-request"]) {
    check(
      "pairing: pair-confirm-request has exactly the 4 canonical fields",
      keySetEq(vectors["pair-confirm-request"], PAIR_CONFIRM_KEYS),
      null
    );
  }
  if (vectors["pair-confirm-response"]) {
    const r = vectors["pair-confirm-response"];
    const keys = Object.keys(r);
    check(
      "pairing: pair-confirm-response has required fields (refresh_token optional) and no extras",
      ["token", "expires_in", "device_id", "server_id"].every((k) => keys.includes(k)) &&
        keys.every((k) => PAIR_CONFIRM_RESPONSE_KEYS.includes(k)),
      null
    );
  }
  if (vectors["pair-confirm-request"]) {
    check(
      "pairing: pair-confirm-request pin matches ^[0-9]{6}$",
      typeof vectors["pair-confirm-request"].pin === "string" && /^[0-9]{6}$/.test(vectors["pair-confirm-request"].pin),
      null
    );
  }

  // 7. QR payload checks.
  if (vectors["qr-payload"]) {
    const qr = vectors["qr-payload"];
    check(
      "qr: uri starts with michi://pair?format=michi-link-pairing&version=1",
      typeof qr.uri === "string" && qr.uri.startsWith("michi://pair?format=michi-link-pairing&version=1"),
      null
    );
    check(
      "qr: endpoint matches ^https?://",
      typeof qr.endpoint === "string" && /^https?:\/\//.test(qr.endpoint),
      null
    );
    check(
      "qr: session_id is a UUID",
      typeof qr.session_id === "string" && UUID_RE.test(qr.session_id),
      null
    );
  }

  // Note: vectors are static snapshots with frozen timestamps; no freshness
  // window is applied to them by design.

  const total = passed + failed;
  console.log(`\n${total} checks: ${passed} passed, ${failed} failed`);
  process.exit(failed > 0 ? 1 : 0);
}

main();
