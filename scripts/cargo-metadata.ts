import {
  parseJsonRecord,
  requireList,
  requireRecord,
  requireString,
  requireStrings,
} from "./proof-json.ts";

export function cargoPackages(metadata: unknown) {
  const root = requireRecord(metadata, "Cargo metadata");
  return requireList(root.packages, "Cargo packages").map((value) => {
    const pkg = requireRecord(value, "Cargo package");
    return {
      ...pkg,
      name: requireString(pkg.name, "crate name"),
      version: requireString(pkg.version, "crate version"),
      manifest_path: requireString(pkg.manifest_path, "crate manifest path"),
      source: pkg.source === null ? null : requireString(pkg.source, "crate source"),
    };
  });
}

export function readCargoInventory(text: string) {
  const root = parseJsonRecord(text);
  return {
    members: requireStrings(root.workspace_members, "workspace members"),
    packages: cargoPackages(root),
  };
}
