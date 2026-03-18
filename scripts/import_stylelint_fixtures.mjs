#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import vm from "node:vm";

const DEFAULT_SUITE_MAP = "tests/compat/stylelint/suite-map.json";
const DEFAULT_SOURCE_ROOT = "tests/compat/stylelint/upstream";
const DEFAULT_OUTPUT_ROOT = "tests/compat/stylelint/imported";

function parseArgs(argv) {
  const options = {
    suiteMap: DEFAULT_SUITE_MAP,
    sourceRoot: DEFAULT_SOURCE_ROOT,
    outputRoot: DEFAULT_OUTPUT_ROOT,
    check: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--check") {
      options.check = true;
      continue;
    }

    if (arg === "--suite-map") {
      index += 1;
      options.suiteMap = argv[index];
      continue;
    }

    if (arg === "--source-root") {
      index += 1;
      options.sourceRoot = argv[index];
      continue;
    }

    if (arg === "--output-root") {
      index += 1;
      options.outputRoot = argv[index];
      continue;
    }

    throw new Error(`Unknown argument: ${arg}`);
  }

  return options;
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function resolveInputPath(candidate, repoRoot, relativeTo) {
  const direct = path.resolve(relativeTo, candidate);
  if (fs.existsSync(direct)) {
    return direct;
  }

  const fromRoot = path.resolve(repoRoot, candidate);
  if (fs.existsSync(fromRoot)) {
    return fromRoot;
  }

  throw new Error(`Failed to resolve path '${candidate}'`);
}

function collectTestRuleCalls(source, sourcePath) {
  const calls = [];
  const sandbox = {
    testRule(spec) {
      calls.push(spec);
    },
  };
  sandbox.globalThis = sandbox;

  vm.createContext(sandbox);
  const script = new vm.Script(source, { filename: sourcePath });
  script.runInContext(sandbox, { timeout: 1000 });

  return calls;
}

function slugify(input) {
  return input
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 80);
}

function normalizeCase(rawCase, context) {
  if (!rawCase || typeof rawCase.code !== "string") {
    throw new Error(
      `Case in ${context.stylelintRule} must include string code for '${context.description}'`,
    );
  }

  const diagnostics =
    context.kind === "reject"
      ? [
          {
            severity: context.level,
            messageContains: String(rawCase.message ?? ""),
            line: Number(rawCase.line ?? 1),
            column: Number(rawCase.column ?? 1),
          },
        ]
      : [];

  const expected = {
    diagnostics,
    fixed: typeof rawCase.fixed === "string" ? rawCase.fixed : null,
  };

  const normalized = {
    id: context.caseId,
    kind: context.kind,
    fast: rawCase.fast === true,
    input: rawCase.code,
    expected,
    source: {
      description: context.description,
      testRuleIndex: context.testRuleIndex,
      caseIndex: context.caseIndex,
    },
  };

  if (typeof rawCase.skipReason === "string") {
    normalized.skip = {
      reasonCode: rawCase.skipReason,
      note: String(rawCase.skipNote ?? ""),
    };
  }

  return normalized;
}

function normalizeSuiteCases(calls, suite, level) {
  const idCounts = new Map();
  const cases = [];

  calls.forEach((call, testRuleIndex) => {
    const entries = [
      ["accept", call.accept],
      ["reject", call.reject],
    ];

    entries.forEach(([kind, list]) => {
      if (!Array.isArray(list)) {
        return;
      }

      list.forEach((rawCase, caseIndex) => {
        const description = String(
          rawCase?.description ?? `${kind}-${testRuleIndex}-${caseIndex}`,
        );
        const baseId = slugify(description) || `${kind}-${testRuleIndex}-${caseIndex}`;
        const seen = idCounts.get(baseId) ?? 0;
        idCounts.set(baseId, seen + 1);
        const caseId = seen === 0 ? baseId : `${baseId}-${seen + 1}`;

        cases.push(
          normalizeCase(rawCase, {
            kind,
            caseId,
            description,
            level,
            stylelintRule: suite.stylelintRule,
            testRuleIndex,
            caseIndex,
          }),
        );
      });
    });
  });

  cases.sort((left, right) => left.id.localeCompare(right.id));
  return cases;
}

function serializeJson(value) {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const repoRoot = process.cwd();
  const suiteMapPath = path.resolve(repoRoot, options.suiteMap);
  const sourceRoot = path.resolve(repoRoot, options.sourceRoot);
  const outputRoot = path.resolve(repoRoot, options.outputRoot);

  const suiteMap = readJson(suiteMapPath);
  if (!Array.isArray(suiteMap.suites)) {
    throw new Error(`Suite map missing 'suites' array in ${suiteMapPath}`);
  }

  const sourcePinPath = resolveInputPath(
    String(suiteMap.sourcePin ?? "tests/compat/stylelint/source-pin.json"),
    repoRoot,
    path.dirname(suiteMapPath),
  );
  const sourcePin = readJson(sourcePinPath);

  fs.mkdirSync(outputRoot, { recursive: true });

  const suites = [...suiteMap.suites].sort((left, right) =>
    String(left.stylelintRule).localeCompare(String(right.stylelintRule)),
  );

  const driftFiles = [];

  for (const suite of suites) {
    const sourcePath = path.join(sourceRoot, suite.sourceFile);
    const source = fs.readFileSync(sourcePath, "utf8");
    const calls = collectTestRuleCalls(source, sourcePath);
    const level = String(suite.level ?? "error");
    const cases = normalizeSuiteCases(calls, suite, level);

    const fixture = {
      schemaVersion: 1,
      stylelint: {
        repository: sourcePin.repository,
        commit: sourcePin.commit,
        rule: suite.stylelintRule,
        sourceFile: suite.sourceFile,
      },
      csslintRule: suite.csslintRule,
      level,
      importMode: suite.importMode,
      supportedOptionSubsets: suite.supportedOptionSubsets ?? [],
      cases,
    };

    const fileName = `${suite.stylelintRule}.json`;
    const outputPath = path.join(outputRoot, fileName);
    const next = serializeJson(fixture);

    if (options.check) {
      const previous = fs.existsSync(outputPath)
        ? fs.readFileSync(outputPath, "utf8")
        : "";
      if (previous !== next) {
        driftFiles.push(path.relative(repoRoot, outputPath));
      }
      continue;
    }

    fs.writeFileSync(outputPath, next);
    const relativeOutput = path.relative(repoRoot, outputPath);
    console.log(`updated ${relativeOutput}`);
  }

  if (options.check && driftFiles.length > 0) {
    throw new Error(
      `Generated fixtures are stale: ${driftFiles.join(", ")}. Run scripts/import_stylelint_fixtures.mjs.`,
    );
  }
}

try {
  main();
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}
