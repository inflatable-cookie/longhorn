import { describe, expect, test } from "bun:test";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import ts from "typescript";

const sourceRoot = new URL("../src/", import.meta.url).pathname;

/**
 * Every validation module in the package.
 *
 * Discovered rather than listed: a new domain must inherit the check, and a
 * list would let it opt out by omission.
 */
function validationModules(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) return validationModules(path);
    return /^validation.*\.ts$/.test(entry) ? [path] : [];
  });
}

/**
 * Parameter names that mean "the keys this object may have".
 *
 * The type alone is not enough to tell the two lists apart. A discriminant
 * check takes `readonly string[]` too -- `member(value, values, path)`,
 * `oneOf`, `assertKnownKind(value, known, …)`, `responseWithStatus(…,
 * statuses)` -- and every one of those *should* be a literal where no
 * generated constant exists. A check that fired on them would be turned off,
 * which is worse than not having it, so the discriminant names are excluded
 * deliberately rather than by omission.
 */
const KEY_PARAMETERS = new Set(["allowed", "expected", "keys"]);

/**
 * Which parameter of which local helper takes an allowed-keys list.
 *
 * Read from each module's own signatures rather than hard-coded, because the
 * helper names differ per domain -- `record`, `keys`, `exact`, `commandBase`,
 * `baseRequest` -- and a hard-coded set would silently stop covering a domain
 * that renamed one.
 */
function keyTakingParameters(source: ts.SourceFile): Map<string, number> {
  const found = new Map<string, number>();
  const visit = (node: ts.Node): void => {
    if (ts.isFunctionDeclaration(node) && node.name) {
      const index = node.parameters.findIndex((parameter) => {
        const text = parameter.type?.getText(source) ?? "";
        if (!/^readonly string\[\]$/.test(text.replace(/\s+/g, " ").trim())) return false;
        return ts.isIdentifier(parameter.name) && KEY_PARAMETERS.has(parameter.name.text);
      });
      if (index !== -1) found.set(node.name.text, index);
    }
    ts.forEachChild(node, visit);
  };
  visit(source);
  return found;
}

interface Literal {
  readonly module: string;
  readonly helper: string;
  readonly line: number;
  readonly text: string;
}

function literalKeyArguments(path: string): Literal[] {
  const source = ts.createSourceFile(
    path,
    readFileSync(path, "utf8"),
    ts.ScriptTarget.ESNext,
    true,
  );
  const parameters = keyTakingParameters(source);
  const found: Literal[] = [];
  const visit = (node: ts.Node): void => {
    if (ts.isCallExpression(node) && ts.isIdentifier(node.expression)) {
      const index = parameters.get(node.expression.text);
      const argument = index === undefined ? undefined : node.arguments[index];
      if (argument !== undefined && ts.isArrayLiteralExpression(argument)) {
        found.push({
          module: path.slice(sourceRoot.length),
          helper: node.expression.text,
          line: source.getLineAndCharacterOfPosition(argument.getStart(source)).line + 1,
          text: argument.getText(source),
        });
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(source);
  return found;
}

describe("allowed keys come from the generated map", () => {
  test("the check recognises a literal in the allowed-keys position", () => {
    // Proves the scan bites. Without this, a bug in the parameter matching
    // reads exactly like a clean codebase.
    const planted = join(sourceRoot, "..", "tests", "fixtures", "planted-key-list.ts");
    expect(literalKeyArguments(planted).map((found) => found.helper)).toEqual(["keys"]);
  });

  test("no validation module passes a literal array as an allowed-keys argument", () => {
    const modules = validationModules(sourceRoot);
    expect(modules.length).toBeGreaterThan(0);

    const literals = modules.flatMap(literalKeyArguments);
    const report = literals
      .map((found) => `${found.module}:${found.line} ${found.helper}(… ${found.text})`)
      .join("\n");

    // Scoped to the argument position, not to array literals generally: a
    // validator legitimately writes `["preset", "replacement"]` as a
    // *discriminant* list, and a check that fired on those would be turned off.
    expect(
      report,
      "Use the domain's generated variant map -- `<DOMAIN>_VARIANT_FIELDS` keyed " +
        "by type then discriminant -- rather than a hand-written key list. A " +
        "literal drifts from the enum silently, which is what Cards 187 to 194 " +
        "removed.",
    ).toBe("");
  });
});
