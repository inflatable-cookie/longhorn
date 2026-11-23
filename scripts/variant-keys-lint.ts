import ts from "typescript";

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
        if (!/^(?:readonlystring\[\]|ReadonlyArray<string>)$/.test(text.replaceAll(/\s+/g, ""))) {
          return false;
        }
        return ts.isIdentifier(parameter.name) && KEY_PARAMETERS.has(parameter.name.text);
      });
      if (index !== -1) {
        found.set(node.name.text, index);
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(source);
  return found;
}

interface RuleContext {
  readonly filename: string;
  readonly sourceCode: { readonly text: string };
  report(diagnostic: {
    readonly message: string;
    readonly loc: { readonly line: number; readonly column: number };
  }): void;
}

export default {
  meta: { name: "longhorn" },
  rules: {
    "generated-variant-keys": {
      create(context: RuleContext) {
        return {
          Program() {
            const source = ts.createSourceFile(
              context.filename,
              context.sourceCode.text,
              ts.ScriptTarget.Latest,
              true,
            );
            const parameters = keyTakingParameters(source);
            const visit = (node: ts.Node): void => {
              if (ts.isCallExpression(node) && ts.isIdentifier(node.expression)) {
                const index = parameters.get(node.expression.text);
                const argument = index === undefined ? undefined : node.arguments[index];
                if (argument && ts.isArrayLiteralExpression(argument)) {
                  const { line, character } = source.getLineAndCharacterOfPosition(
                    argument.getStart(source),
                  );
                  context.report({
                    message: "Use the generated variant field map for allowed keys.",
                    loc: { line: line + 1, column: character },
                  });
                }
              }
              ts.forEachChild(node, visit);
            };
            visit(source);
          },
        };
      },
    },
  },
};
