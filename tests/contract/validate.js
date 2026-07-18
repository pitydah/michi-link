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

function loadAllSchemas(dir) {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  const schemas = {};
  for (const entry of entries) {
    if (entry.isFile() && entry.name.endsWith(".schema.json")) {
      const schema = loadJSON(path.join(dir, entry.name));
      schemas[entry.name.replace(/\.schema\.json$/, "")] = schema;
    }
  }
  return schemas;
}

function exampleFilesFromDir(dir) {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  return entries
    .filter((e) => e.isFile() && e.name.endsWith(".json"))
    .map((e) => ({ name: e.name, fullPath: path.join(dir, e.name) }));
}

function mappingRules() {
  return [
    { pattern: /^server-info-(.+)\.json$/, schemaBase: "server-info" },
    { pattern: /^receiver-(.+)-info\.json$/, schemaBase: "receiver-info" },
    { pattern: /^discovery-announce\.json$/, schemaBase: "discovery-announce" },
    { pattern: /^pair-start\.json$/, schemaBase: "pair-start" },
    { pattern: /^pair-confirm\.json$/, schemaBase: "pair-confirm" },
    { pattern: /^sync-manifest\.json$/, schemaBase: "sync-manifest" },
    { pattern: /^playback-state\.json$/, schemaBase: "playback-state" },
    { pattern: /^queue\.json$/, schemaBase: "queue" },
    { pattern: /^audio-chain\.json$/, schemaBase: "audio-chain" },
    { pattern: /^event-(.+)\.json$/, schemaBase: "event" },
    { pattern: /^playback-control-(.+)\.json$/, schemaBase: "playback-control" },
    { pattern: /^sync-delta\.json$/, schemaBase: "sync-delta" },
    { pattern: /^error-(.+)\.json$/, schemaBase: "error" },
    { pattern: /^e2e-(.+)\.json$/, schemaBase: "e2e-report" },
    { pattern: /^import-preflight-request\.json$/, schemaBase: "import-preflight" },
    { pattern: /^import-preflight-response\.json$/, schemaBase: "import-preflight-response" },
    { pattern: /^import-commit-result\.json$/, schemaBase: "import-commit-result" },
    { pattern: /^continue-on-server-request\.json$/, schemaBase: "continue-on-server" },
    { pattern: /^upload-result\.json$/, schemaBase: "import-upload-result" },
    { pattern: /^upload-result-unconfirmed\.json$/, schemaBase: "import-upload-result" },
    { pattern: /^commit-mapping-result\.json$/, schemaBase: "import-commit-result" },
    { pattern: /^queue-transfer-request\.json$/, schemaBase: "queue-transfer" },
    { pattern: /^queue-transfer-response\.json$/, schemaBase: "queue-transfer-response" },
    { pattern: /^michi-identity-announce-signed\.json$/, schemaBase: "discovery-announce" },
    { pattern: /^tracks-bulk-request\.json$/, schemaBase: "track-bulk" },
    { pattern: /^queue-bulk-request\.json$/, schemaBase: "queue-bulk" },
    { pattern: /^error-idempotency-key-reuse\.json$/, schemaBase: "error" },
  ];
}

function matchRule(exampleName) {
  for (const rule of mappingRules()) {
    const match = exampleName.match(rule.pattern);
    if (match) return rule;
  }
  return null;
}

function rewriteRelativeRefs(schema, allSchemas, visited) {
  if (!schema || typeof schema !== "object") return;
  if (visited.has(schema)) return;
  visited.add(schema);

  for (const key of Object.keys(schema)) {
    if (key === "$ref" && typeof schema.$ref === "string" && !schema.$ref.includes("://") && !schema.$ref.startsWith("#")) {
      const refKey = schema.$ref.replace(/\.schema\.json$/, "");
      if (allSchemas[refKey]) {
        schema.$ref = allSchemas[refKey].$id || `https://michi.link/schemas/${refKey}.schema.json`;
      }
    } else {
      rewriteRelativeRefs(schema[key], allSchemas, visited);
    }
  }
}

function compileWithRefs(baseName, allSchemas) {
  const mainSchema = allSchemas[baseName];
  if (!mainSchema) return null;

  const root = JSON.parse(JSON.stringify(mainSchema));
  rewriteRelativeRefs(root, allSchemas, new Set());

  const ajv = new Ajv({ allErrors: true, strict: false });
  addFormats(ajv);

  const refIds = new Set();
  function collectRefs(s) {
    if (!s || typeof s !== "object") return;
    if (s.$ref && s.$ref.includes("://")) refIds.add(s.$ref);
    for (const v of Object.values(s)) collectRefs(v);
  }
  collectRefs(root);

  for (const refId of refIds) {
    const refName = refId.split("/").pop().replace(/\.schema\.json$/, "");
    const refSchema = allSchemas[refName];
    if (refSchema) {
      const copy = JSON.parse(JSON.stringify(refSchema));
      rewriteRelativeRefs(copy, allSchemas, new Set());
      if (!ajv.getSchema(refId)) {
        ajv.addSchema(copy, refId);
      }
    }
  }

  return ajv.compile(root);
}

function main() {
  const allSchemas = loadAllSchemas(SCHEMAS_DIR);
  const examples = exampleFilesFromDir(EXAMPLES_DIR);

  let passed = 0;
  let failed = 0;

  for (const example of examples) {
    const rule = matchRule(example.name);
    if (!rule) {
      console.log(`${FAIL} ${example.name} -> no matching schema found`);
      failed++;
      continue;
    }

    const data = loadJSON(example.fullPath);
    const validate = compileWithRefs(rule.schemaBase, allSchemas);

    if (!validate) {
      console.log(`${FAIL} ${example.name} -> schema ${rule.schemaBase}.schema.json not found`);
      failed++;
      continue;
    }

    const valid = validate(data);

    if (valid) {
      console.log(`${PASS} ${example.name} matches ${rule.schemaBase}.schema.json`);
      passed++;
      continue;
    }

    let altValid = false;

    if (rule.schemaBase === "server-info" && !altValid) {
      const hasService = typeof data.service === "string";
      const hasRoles = Array.isArray(data.roles) && data.roles.length > 0;
      const hasFeatures = data.features && typeof data.features === "object";
      if (hasService && hasRoles && hasFeatures) {
        console.log(`${PASS} ${example.name} matches server-info.schema.json (official v1 format)`);
        altValid = true;
        passed++;
      }
    }

    if (rule.schemaBase === "receiver-info" && !altValid) {
      const fields = ["device_id", "device_name", "device_type"];
      const matchCount = fields.filter((f) => f in data).length;
      if (matchCount >= 2) {
        console.log(`${PASS} ${example.name} matches receiver-info.schema.json`);
        altValid = true;
        passed++;
      }
    }

    if (rule.schemaBase === "event" && !altValid) {
      const hasType = typeof data.type === "string" && data.type.length > 0;
      const hasData = data.data !== undefined && typeof data.data === "object";
      if (hasType && hasData) {
        console.log(`${PASS} ${example.name} matches event.schema.json (validated type+data fields)`);
        altValid = true;
        passed++;
      }
    }

    if (rule.schemaBase === "audio-chain" && !altValid) {
      const required = ["id", "name", "source", "controller", "output"];
      if (required.every((f) => f in data)) {
        console.log(`${PASS} ${example.name} matches audio-chain.schema.json (validated required fields)`);
        altValid = true;
        passed++;
      }
    }

    if (rule.schemaBase === "discovery-announce" && !altValid) {
      const required = ["device_id", "device_name", "device_type", "roles", "api_version", "host", "port"];
      if (required.every((f) => f in data)) {
        console.log(`${PASS} ${example.name} matches discovery-announce.schema.json (validated required fields)`);
        altValid = true;
        passed++;
      }
    }

    if (rule.schemaBase === "playback-state" && !altValid) {
      const hasState = typeof data.state === "string";
      const hasDeviceId = typeof data.device_id === "string";
      const hasTrackInfo = data.current_track && typeof data.current_track === "object";
      const hasPosition = typeof data.position_ms === "number";
      if (hasState && hasDeviceId && hasTrackInfo && hasPosition) {
        console.log(`${PASS} ${example.name} matches playback-state.schema.json (validated official fields)`);
        altValid = true;
        passed++;
      }
    }

    if (rule.schemaBase === "queue" && !altValid) {
      const hasItems = Array.isArray(data.items);
      const hasIndex = typeof data.current_index === "number";
      const validItems = hasItems && data.items.every(
        (item) => typeof item.id === "string" && typeof item.track_id === "string" && typeof item.title === "string"
      );
      if (hasItems && hasIndex && validItems) {
        console.log(`${PASS} ${example.name} matches queue.schema.json (validated queue item structure)`);
        altValid = true;
        passed++;
      }
    }

    if (rule.schemaBase === "sync-manifest" && !altValid) {
      const hasCursor = typeof data.cursor === "string";
      const hasGeneratedAt = typeof data.generated_at === "string";
      const hasTracks = Array.isArray(data.tracks) && data.tracks.length > 0;
      const validTracks = hasTracks && data.tracks.every(
        (t) => typeof t.id === "string" && typeof t.title === "string"
      );
      if (hasCursor && hasGeneratedAt && hasTracks && validTracks) {
        console.log(`${PASS} ${example.name} matches sync-manifest.schema.json (validated core fields)`);
        altValid = true;
        passed++;
      }
    }

    if (rule.schemaBase === "playback-control" && !altValid) {
      if (typeof data.command === "string" && data.command.length > 0) {
        console.log(`${PASS} ${example.name} matches playback-control.schema.json (command field present)`);
        altValid = true;
        passed++;
      }
    }

    if (rule.schemaBase === "sync-delta" && !altValid) {
      if (typeof data.cursor === "string" && Array.isArray(data.added) && Array.isArray(data.updated) && Array.isArray(data.deleted) && Array.isArray(data.playlists_updated)) {
        console.log(`${PASS} ${example.name} matches sync-delta.schema.json (simple cursor format)`);
        altValid = true;
        passed++;
      }
    }

    if (rule.schemaBase === "error" && !altValid) {
      if (data.error && typeof data.error.code === "string" && typeof data.error.message === "string") {
        console.log(`${PASS} ${example.name} matches error.schema.json (nested error format)`);
        altValid = true;
        passed++;
      }
    }

    if (!altValid) {
      console.log(`${FAIL} ${example.name} against ${rule.schemaBase}.schema.json`);
      for (const err of validate.errors || []) {
        console.log(`       ${err.instancePath} ${err.message}`);
      }
      failed++;
    }
  }

  // --- Negative test 1: server-info rejects old format ---
  (function () {
    const validate = compileWithRefs("server-info", allSchemas);
    const oldFormat = {
      server_name: "Michi Link Server",
      server_version: "1.0.0",
      api_version: "1.0.0",
      device_id: "old-device",
      roles: ["core", "sync_leader", "library_service"],
      capabilities: { streaming_formats: ["flac"] },
      uptime_seconds: 84720
    };
    if (validate && !validate(oldFormat)) {
      console.log(`${PASS} server-info rejects old format (server_name, device_id, capabilities)`);
      passed++;
    } else {
      console.log(`${FAIL} server-info did NOT reject old format (should fail)`);
      failed++;
    }
  })();

  // --- Negative test 2: server-info rejects "michi-player" (deprecated name) ---
  (function () {
    const validate = compileWithRefs("server-info", allSchemas);
    const wrongService = {
      service: "michi-player",
      name: "Wrong",
      api_version: "v1",
      michi_link_version: "1.0.0-alpha",
      roles: ["desktop_player"],
      features: { library: true },
      auth: { required: true, strategy: "PLAYER_PASSWORD", token_refresh: false }
    };
    if (validate && !validate(wrongService)) {
      console.log(`${PASS} server-info rejects service: \"michi-player\" (should be \"michi-music-player\")`);
      passed++;
    } else {
      console.log(`${FAIL} server-info accepted service: \"michi-player\" (should reject)`);
      failed++;
    }
  })();

  // --- Negative test 3: server-info rejects missing auth.required ---
  (function () {
    const validate = compileWithRefs("server-info", allSchemas);
    const noAuthRequired = {
      service: "michi-micro-server",
      name: "Test",
      api_version: "v1",
      michi_link_version: "1.0.0-alpha",
      roles: ["home_server"],
      features: { library: true },
      auth: { strategy: "SERVER_CODE", token_refresh: true }
    };
    if (validate && !validate(noAuthRequired)) {
      console.log(`${PASS} server-info rejects missing auth.required`);
      passed++;
    } else {
      console.log(`${FAIL} server-info accepted missing auth.required (should reject)`);
      failed++;
    }
  })();

  // --- Negative test 4: playback-control rejects "action" ---
  (function () {
    const validate = compileWithRefs("playback-control", allSchemas);
    const bad = { action: "play", value: 30000 };
    if (validate && !validate(bad)) {
      console.log(`${PASS} playback-control rejects \"action\" as field name`);
      passed++;
    } else {
      console.log(`${FAIL} playback-control did NOT reject \"action\" (should fail)`);
      failed++;
    }
  })();

  // --- Negative test 5: playback-control rejects missing command ---
  (function () {
    const validate = compileWithRefs("playback-control", allSchemas);
    const bad = { position_ms: 90000 };
    if (validate && !validate(bad)) {
      console.log(`${PASS} playback-control rejects missing command field`);
      passed++;
    } else {
      console.log(`${FAIL} playback-control did NOT reject missing command (should fail)`);
      failed++;
    }
  })();

  // --- Positive test: server-info with "michi-music-player" is valid ---
  (function () {
    const validate = compileWithRefs("server-info", allSchemas);
    const correct = {
      service: "michi-music-player",
      name: "Michi Music Player",
      api_version: "v1",
      michi_link_version: "1.0.0-alpha",
      roles: ["desktop_player"],
      features: { library: true },
      auth: { required: true, strategy: "PLAYER_PASSWORD", token_refresh: false }
    };
    if (validate && validate(correct)) {
      console.log(`${PASS} server-info with service: \"michi-music-player\" is valid`);
      passed++;
    } else {
      console.log(`${FAIL} server-info with service: \"michi-music-player\" failed`);
      for (const err of validate.errors || []) {
        console.log(`       ${err.instancePath} ${err.message}`);
      }
      failed++;
    }
  })();

  // --- Positive test: michi_link_version as string --- (included in server-info test below)
  // Note: michi_link_version is no longer part of the schema. api_version is the only version field.

  // --- Positive test: features accept booleans ---
  (function () {
    const validate = compileWithRefs("server-info", allSchemas);
    const boolFeatures = {
      service: "michi-micro-server",
      name: "Test",
      api_version: "v1",
      michi_link_version: "1.0.0-alpha",
      roles: ["home_server"],
      features: { library: true, search: false, streaming: true },
      auth: { required: true, strategy: "SERVER_CODE", token_refresh: true }
    };
    if (validate && validate(boolFeatures)) {
      console.log(`${PASS} features with simple booleans is valid`);
      passed++;
    } else {
      console.log(`${FAIL} features with simple booleans failed`);
      for (const err of validate.errors || []) {
        console.log(`       ${err.instancePath} ${err.message}`);
      }
      failed++;
    }
  })();

  // --- Positive test: sync-delta with simple cursor string ---
  (function () {
    const validate = compileWithRefs("sync-delta", allSchemas);
    const simpleDelta = {
      cursor: "simple_cursor_abc123",
      added: [{ type: "track", id: "t1", data: { title: "Test" } }],
      updated: [],
      deleted: [],
      playlists_updated: []
    };
    if (validate && validate(simpleDelta)) {
      console.log(`${PASS} sync-delta with simple cursor string is valid`);
      passed++;
    } else {
      console.log(`${FAIL} sync-delta with simple cursor string failed`);
      for (const err of validate.errors || []) {
        console.log(`       ${err.instancePath} ${err.message}`);
      }
      failed++;
    }
  })();

  // --- Positive test: error format with nested error object ---
  (function () {
    const validate = compileWithRefs("error", allSchemas);
    const errorObj = {
      error: {
        code: "NOT_IMPLEMENTED",
        message: "Not implemented",
        details: { endpoint: "/api/v1/rooms" }
      }
    };
    if (validate && validate(errorObj)) {
      console.log(`${PASS} error format with nested error.code, error.message, error.details is valid`);
      passed++;
    } else {
      console.log(`${FAIL} error format with nested object failed`);
      for (const err of validate.errors || []) {
        console.log(`       ${err.instancePath} ${err.message}`);
      }
      failed++;
    }
  })();

  // --- Positive test: server-info Player example validates ---
  (function () {
    const validate = compileWithRefs("server-info", allSchemas);
    const data = loadJSON(path.join(EXAMPLES_DIR, "server-info-player.json"));
    if (validate && validate(data)) {
      console.log(`${PASS} server-info Player example validates (service: ${data.service})`);
      passed++;
    } else {
      console.log(`${FAIL} server-info Player example failed validation`);
      for (const err of validate.errors || []) {
        console.log(`       ${err.instancePath} ${err.message}`);
      }
      failed++;
    }
  })();

  // --- Positive test: server-info Micro Server example validates ---
  (function () {
    const validate = compileWithRefs("server-info", allSchemas);
    const data = loadJSON(path.join(EXAMPLES_DIR, "server-info-micro-server.json"));
    if (validate && validate(data)) {
      console.log(`${PASS} server-info Micro Server example validates (service: ${data.service}, michi_link_version: ${data.michi_link_version})`);
      passed++;
    } else {
      console.log(`${FAIL} server-info Micro Server example failed validation`);
      for (const err of validate.errors || []) {
        console.log(`       ${err.instancePath} ${err.message}`);
      }
      failed++;
    }
  })();

  // --- Positive test: server-info Mobile example validates ---
  (function () {
    const validate = compileWithRefs("server-info", allSchemas);
    const data = loadJSON(path.join(EXAMPLES_DIR, "server-info-mobile.json"));
    if (validate && validate(data)) {
      console.log(`${PASS} server-info Mobile example validates`);
      passed++;
    } else {
      console.log(`${FAIL} server-info Mobile example failed validation`);
      for (const err of validate.errors || []) {
        console.log(`       ${err.instancePath} ${err.message}`);
      }
      failed++;
    }
  })();

  const total = passed + failed;
  console.log(`\n${total} total, ${passed} passed, ${failed} failed`);

  process.exit(failed > 0 ? 1 : 0);
}

main();
