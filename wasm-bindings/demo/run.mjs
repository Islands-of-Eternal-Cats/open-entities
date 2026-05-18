import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { Simulation } from "../pkg/open_entities_wasm.js";

const fixturePath = join(
  dirname(fileURLToPath(import.meta.url)),
  "../../fixtures/spawn_entity_templates.yaml",
);
const yaml = readFileSync(fixturePath, "utf8");

const sim = new Simulation();
console.log(sim.hello());

sim.loadTemplatesYaml(yaml);

const names = ["marker", "heavy_tank", "tank", "scout", "unit"];
const spawnedIds = [];

for (const name of names) {
  const overrides =
    name === "scout"
      ? { position: { x: 50.0, y: 25.0 }, health: { current: 40, max: 100 } }
      : {};
  const spawned = sim.spawnEntity(name, overrides);
  console.log(`spawned ${name}`, spawned.index, spawned.generation);
  spawnedIds.push({ index: spawned.index, generation: spawned.generation });
}

const json = sim.getWorldAsJson();
const parsed = JSON.parse(json);

if (parsed.version !== 3) {
  throw new Error(`expected version 3, got ${parsed.version}`);
}
if (!Array.isArray(parsed.entities) || parsed.entities.length !== 5) {
  throw new Error(`expected 5 entities, got ${parsed.entities?.length}`);
}

const scout = parsed.entities.find((e) => e.entity_type === "scout");
if (!scout) {
  throw new Error("scout entity missing from export");
}
if (scout.position?.x !== 50.0 || scout.position?.y !== 25.0) {
  throw new Error(`scout position override missing: ${JSON.stringify(scout.position)}`);
}
if (scout.health?.current !== 40 || scout.health?.max !== 100) {
  throw new Error(`scout health override missing: ${JSON.stringify(scout.health)}`);
}

for (const { index, generation } of spawnedIds) {
  const found = parsed.entities.some(
    (e) => e.id?.index === index && e.id?.generation === generation,
  );
  if (!found) {
    throw new Error(`spawned id ${index}/${generation} not found in export`);
  }
}

console.log("wasm spawn demo ok");
