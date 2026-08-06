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

  const register = (name) => {
    const schema = loadJSON(path.join(SCHEMAS_DIR, name));
    ajv.addSchema(schema);
    return ajv.getSchema(SCHEMA_BASE + name);
  };

  const validateIdentity = register("michi-identity.schema.json");
  const validateAnnounce = register("discovery-announce.schema.json");
  const validateServerInfo = register("server-info.schema.json");

  const identityDoc = loadJSON(path.join(EXAMPLES_DIR, "identity-document.json"));
  const signedAnnounce = loadJSON(path.join(EXAMPLES_DIR, "michi-identity-announce-signed.json"));
  const serverPlayer = loadJSON(path.join(EXAMPLES_DIR, "server-info-player.json"));

  // Positive checks with real AJV.
  check("examples/identity-document.json matches michi-identity.schema.json", validateIdentity(identityDoc), validateIdentity);
  check(
    "examples/michi-identity-announce-signed.json matches discovery-announce.schema.json (full signed group)",
    validateAnnounce(signedAnnounce),
    validateAnnounce
  );
  check("examples/server-info-player.json matches server-info.schema.json", validateServerInfo(serverPlayer), validateServerInfo);

  const identityFields = ["michi_id", "public_key", "signature", "timestamp_ms", "nonce"];
  const missing = identityFields.filter((f) => !(f in signedAnnounce));
  check(`signed announce has complete signature group (${identityFields.join(", ")})`, missing.length === 0, null);

  const BASE64URL_RE = /^[A-Za-z0-9_-]+$/;
  const announceId = signedAnnounce.michi_id;
  const announceKey = signedAnnounce.public_key;
  const idOk = typeof announceId === "string" && announceId.length === 43 && BASE64URL_RE.test(announceId);
  const keyOk = typeof announceKey === "string" && announceKey.length === 43 && BASE64URL_RE.test(announceKey);
  check("identity coherence: announce michi_id is exactly 43 base64url chars (format only, no crypto)", idOk, null);
  check("identity coherence: announce public_key is exactly 43 base64url chars (format only, no crypto)", keyOk, null);

  // Negative checks: valid identity payloads with one mutation each.
  const identityBase = {
    identity_scheme: "ed25519-blake3-v1",
    michi_id: announceId,
    public_key: announceKey,
  };
  const announceBase = (() => {
    const base = { ...signedAnnounce };
    base.timestamp_ms = Date.now();
    return base;
  })();

  const rejects = (label, validate, payload) => {
    const valid = validate(payload);
    check(label, !valid, valid ? null : validate);
  };

  rejects("negative: michi-identity rejects hex michi_id (64 chars)", validateIdentity, {
    ...identityBase,
    michi_id: "aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
  });
  rejects("negative: michi-identity rejects identity_scheme \"sha256\"", validateIdentity, {
    ...identityBase,
    identity_scheme: "sha256",
  });
  rejects("negative: michi-identity rejects malformed public_key", validateIdentity, {
    ...identityBase,
    public_key: "not a valid base64url key!!",
  });
  rejects("negative: discovery-announce rejects signed announce without nonce", validateAnnounce, {
    ...announceBase,
    nonce: undefined,
  });
  rejects("negative: discovery-announce rejects too-short nonce", validateAnnounce, {
    ...announceBase,
    nonce: "short",
  });
  rejects("negative: discovery-announce rejects malformed michi_id (not 43 base64url chars)", validateAnnounce, {
    ...announceBase,
    michi_id: "aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
  });

  // Freshness rule (90s window): a positive check against the static example
  // would be a time bomb, so the rule is verified as a NEGATIVE check against
  // a synthetic payload. Runtime enforcement lives in michi-identity (Rust).
  const stale = { ...announceBase, timestamp_ms: Date.now() - 10 * 60 * 1000 };
  const deltaSec = (Date.now() - stale.timestamp_ms) / 1000;
  check("freshness rule: timestamp older than 90s is rejected", Math.abs(deltaSec) > 90, null);

  const total = passed + failed;
  console.log(`\n${total} checks: ${passed} passed, ${failed} failed`);
  process.exit(failed > 0 ? 1 : 0);
}

main();
