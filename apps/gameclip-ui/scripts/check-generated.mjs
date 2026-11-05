// SPDX-License-Identifier: GPL-3.0-or-later
// ts-rs schreibt die Typen beim `cargo test -p gameclip-protocol` direkt nach src/generated/.
// Dieses Skript kopiert nichts; es prüft nur, dass die erwarteten Dateien da sind.
import { existsSync } from "node:fs";

const needed = ["Command", "Response", "ServiceEvent", "Settings", "ClipEntry", "ServiceStatus"];
const missing = needed.filter((n) => !existsSync(new URL(`../src/generated/${n}.ts`, import.meta.url)));
if (missing.length) {
  console.error(`Fehlende Typen: ${missing.join(", ")}. Erst \`cargo test -p gameclip-protocol\` ausführen.`);
  process.exit(1);
}
console.log("Typen vorhanden.");
