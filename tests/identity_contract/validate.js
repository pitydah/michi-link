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

function main() {
  const ajv = new Ajv({ allErrors: true, strict: false });
  addFormats(ajv);

  let passed = 0;
  let failed = 0;

  // Test 1: michi-identity schema validates the identity subset
  const identitySchema = loadJSON(path.join(SCHEMAS_DIR, "michi-identity.schema.json"));
  const identityExample = loadJSON(path.join(EXAMPLES_DIR, "michi-identity-announce-signed.json"));

  const subset = {
    michi_id: identityExample.michi_id,
    public_key: identityExample.public_key,
    algorithm: "ed25519",
  };

  const validateIdentity = ajv.compile(identitySchema);
  if (validateIdentity(subset)) {
    console.log(`${PASS} michi-identity.schema.json validates identity subset`);
    passed++;
  } else {
    console.log(`${FAIL} michi-identity.schema.json failed`);
    for (const err of validateIdentity.errors) {
      console.log(`       ${err.instancePath} ${err.message}`);
    }
    failed++;
  }

  // Test 2: Legacy announce has required fields for discovery-announce
  const legacyExample = loadJSON(path.join(EXAMPLES_DIR, "discovery-announce.json"));
  const required = ["device_id", "device_name", "device_type", "roles", "api_version", "host", "port"];
  const hasRequired = required.every((f) => f in legacyExample);
  if (hasRequired) {
    console.log(`${PASS} Legacy announce has all required discovery-announce fields`);
    passed++;
  } else {
    console.log(`${FAIL} Legacy announce missing required fields`);
    failed++;
  }

  // Test 3: Signed announce has all required fields plus optional identity fields
  const signedRequired = [...required, "michi_id", "public_key", "signature"];
  const hasSignedRequired = signedRequired.every((f) => f in identityExample);
  if (hasSignedRequired) {
    console.log(`${PASS} Signed announce has required + identity fields`);
    passed++;
  } else {
    const missing = signedRequired.filter((f) => !(f in identityExample));
    console.log(`${FAIL} Signed announce missing: ${missing.join(", ")}`);
    failed++;
  }

  // Test 4: server-info with michi_id: null is valid
  const serverSchema = loadJSON(path.join(SCHEMAS_DIR, "server-info.schema.json"));
  const serverPlayer = loadJSON(path.join(EXAMPLES_DIR, "server-info-player.json"));
  const validateServer = ajv.compile(serverSchema);
  if (validateServer(serverPlayer)) {
    console.log(`${PASS} server-info.schema.json accepts legacy server (no michi_id)`);
    passed++;
  } else {
    console.log(`${FAIL} server-info.schema.json rejected legacy server`);
    for (const err of validateServer.errors) {
      console.log(`       ${err.instancePath} ${err.message}`);
    }
    failed++;
  }

  // Test 5: server-info with identity fields is valid
  const serverWithIdentity = { ...serverPlayer, michi_id: identityExample.michi_id, public_key: identityExample.public_key };
  if (validateServer(serverWithIdentity)) {
    console.log(`${PASS} server-info.schema.json accepts server with michi_id`);
    passed++;
  } else {
    console.log(`${FAIL} server-info.schema.json rejected server with michi_id`);
    for (const err of validateServer.errors) {
      console.log(`       ${err.instancePath} ${err.message}`);
    }
    failed++;
  }

  const total = passed + failed;
  console.log(`\n${total} total, ${passed} passed, ${failed} failed`);

  process.exit(failed > 0 ? 1 : 0);
}

main();
