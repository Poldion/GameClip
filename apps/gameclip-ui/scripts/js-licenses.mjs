// SPDX-License-Identifier: GPL-3.0-or-later
// Schreibt die Lizenzen der JavaScript-Pakete, die in der Oberfläche landen (nur `dependencies`),
// mit vollem Text nach THIRD_PARTY_LICENSES_JS.md im Repository. Die Rust-Abhängigkeiten stehen
// in THIRD_PARTY_LICENSES.md (cargo about).
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";

const ACCEPTED = new Set(["MIT", "Apache-2.0 OR MIT", "Apache-2.0", "ISC", "BSD-2-Clause", "BSD-3-Clause", "0BSD"]);

const checker = new URL("../node_modules/license-checker/bin/license-checker", import.meta.url);
const json = execFileSync(
  process.execPath,
  [checker.pathname.replace(/^\/([A-Za-z]:)/, "$1"), "--production", "--excludePrivatePackages", "--json"],
  { encoding: "utf8", cwd: new URL("..", import.meta.url) },
);
const pkgs = JSON.parse(json);
const out = [
  "# Lizenzen der JavaScript-Pakete",
  "",
  "Diese Pakete sind in die Oberfläche von GameClip eingebettet. Erzeugt mit `npm run licenses` in `apps/gameclip-ui`.",
  "",
];
const bad = [];
for (const [name, info] of Object.entries(pkgs).sort(([a], [b]) => a.localeCompare(b))) {
  const lic = Array.isArray(info.licenses) ? info.licenses.join(" OR ") : info.licenses;
  if (!ACCEPTED.has(lic)) bad.push(`${name}: ${lic}`);
  out.push(`## ${name}`, "", `Lizenz: ${lic}${info.repository ? ` – ${info.repository}` : ""}`, "");
  if (info.licenseFile) {
    out.push("```text", readFileSync(info.licenseFile, "utf8").trim(), "```", "");
  }
}
if (bad.length) {
  console.error(`Nicht freigegebene Lizenzen:\n${bad.join("\n")}`);
  process.exit(1);
}
writeFileSync(new URL("../../../THIRD_PARTY_LICENSES_JS.md", import.meta.url), out.join("\n"));
console.log(`${Object.keys(pkgs).length} Pakete geschrieben.`);
