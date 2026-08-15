import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile, readdir } from "node:fs/promises";

test("documentation inventory is explicit and complete", async () => {
  const docs = (await readdir(new URL("../../docs/", import.meta.url)))
    .filter((name) => name.endsWith(".md"))
    .sort();
  assert.deepEqual(docs, [
    "arquitectura.md",
    "ciclo-redencion.md",
    "gobernanza.md",
    "integracion.md",
    "modelo-economico.md",
    "observabilidad.md",
    "operaciones.md",
  ]);
});

test("README binds the approved CrownDTL banner", async () => {
  const [readme, banner] = await Promise.all([
    readFile(new URL("../../README.md", import.meta.url), "utf8"),
    readFile(new URL("../../assets/banner.png", import.meta.url)),
  ]);
  assert.match(readme, /\.\/assets\/banner\.png/);
  assert.equal(
    createHash("sha256").update(banner).digest("hex"),
    "224ff6a5bcd7bf3c11d6f1a8a2178540fcc7e197f748a7ecd9b7141fd6788a7a",
  );
});

test("promotion workflow verifies the production reference", async () => {
  const workflow = await readFile(
    new URL("../../.github/workflows/release-integrity.yml", import.meta.url),
    "utf8",
  );
  assert.match(workflow, /origin\/production/);
  assert.match(workflow, /GITHUB_REF_TYPE/);
  assert.match(workflow, /github\.event\.release\.tag_name/);
});
