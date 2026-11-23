import { isRecord } from "../packages/longhorn/src/core/record.ts";

export function requireRecord(value: unknown, label: string): Record<string, unknown> {
  if (!isRecord(value)) {
    throw new TypeError(`${label} must be an object`);
  }
  return value;
}

export function parseJsonRecord(text: string): Record<string, unknown> {
  const value: unknown = JSON.parse(text);
  return requireRecord(value, "proof JSON");
}

export function requireString(value: unknown, label: string): string {
  if (typeof value !== "string") {
    throw new TypeError(`${label} must be a string`);
  }
  return value;
}

export function requireList(value: unknown, label: string): ReadonlyArray<unknown> {
  if (!Array.isArray(value)) {
    throw new TypeError(`${label} must be an array`);
  }
  return value;
}

export function requireStrings(value: unknown, label: string): Array<string> {
  return requireList(value, label).map((entry) => requireString(entry, label));
}

function stringMap(value: unknown, label: string): Record<string, string> {
  const table = value === undefined ? {} : requireRecord(value, label);
  return Object.fromEntries(
    Object.entries(table).map(([name, entry]) => [name, requireString(entry, `${label}.${name}`)]),
  );
}

export function readPackageIdentity(text: string) {
  const value = parseJsonRecord(text);
  return {
    name: requireString(value.name, "package name"),
    version: requireString(value.version, "package version"),
  };
}

export function readPackageManifest(text: string) {
  const value = parseJsonRecord(text);
  const meta =
    value.peerDependenciesMeta === undefined
      ? {}
      : requireRecord(value.peerDependenciesMeta, "peerDependenciesMeta");
  return {
    ...value,
    name: requireString(value.name, "package name"),
    version:
      value.version === undefined ? undefined : requireString(value.version, "package version"),
    dependencies:
      value.dependencies === undefined ? undefined : stringMap(value.dependencies, "dependencies"),
    devDependencies:
      value.devDependencies === undefined
        ? undefined
        : stringMap(value.devDependencies, "devDependencies"),
    overrides: value.overrides === undefined ? undefined : stringMap(value.overrides, "overrides"),
    peerDependencies:
      value.peerDependencies === undefined
        ? undefined
        : stringMap(value.peerDependencies, "peerDependencies"),
    peerDependenciesMeta:
      value.peerDependenciesMeta === undefined
        ? undefined
        : Object.fromEntries(
            Object.entries(meta).map(([name, entry]) => [
              name,
              requireRecord(entry, `peerDependenciesMeta.${name}`),
            ]),
          ),
    exports: value.exports === undefined ? undefined : requireRecord(value.exports, "exports"),
  };
}

export function readPermissions(text: string) {
  const value = parseJsonRecord(text);
  return { permissions: requireStrings(value.permissions, "permissions") };
}
