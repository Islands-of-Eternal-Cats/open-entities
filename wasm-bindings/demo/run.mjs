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

if (parsed.version !== 4) {
  throw new Error(`expected version 4, got ${parsed.version}`);
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

// --- Tick demo: scout walks to move_target (fixture pose, no override) ---
const tickSim = new Simulation();
tickSim.loadTemplatesYaml(yaml);
tickSim.spawnEntity("scout", {});

const maxTicks = 1000;
for (let i = 0; i < maxTicks; i++) {
  tickSim.tick(16);
  if (i > 0 && i % 30 === 0) {
    console.log(`tick ${i}`);
  }
}

const tickJson = JSON.parse(tickSim.getWorldAsJson());
const tickScout = tickJson.entities.find((e) => e.entity_type === "scout");
if (!tickScout) {
  throw new Error("tick demo: scout missing");
}
if (tickScout.move_target !== undefined) {
  throw new Error(
    `tick demo: scout still has move_target after ${maxTicks} ticks`,
  );
}
const tx = 20.0;
const ty = 0.0;
const px = tickScout.position?.x;
const py = tickScout.position?.y;
if (Math.abs(px - tx) > 0.01 || Math.abs(py - ty) > 0.01) {
  throw new Error(
    `tick demo: expected position near (${tx}, ${ty}), got (${px}, ${py})`,
  );
}

console.log("wasm tick demo ok");

// --- Transport demo: a unit rides along, then steps off ---
// This is the only check of the *JavaScript* names: the Rust tests call these methods by their
// Rust names, so a typo in a `js_name` would sail past them and land in the browser.
const rideSim = new Simulation();
rideSim.loadTemplatesYaml(yaml);

const truck = rideSim.spawnEntity("scout", { boardable: 4 });
const passenger = rideSim.spawnEntity("marker", {
  position: { x: 10.5, y: 5.0 },
});

if (rideSim.freeSeats(truck) !== 4) {
  throw new Error(`transport demo: expected 4 free seats, got ${rideSim.freeSeats(truck)}`);
}
if (rideSim.freeSeats(passenger) !== null) {
  throw new Error("transport demo: a unit with no seats should report null, not a number");
}

rideSim.board(passenger, truck);
if (rideSim.freeSeats(truck) !== 3) {
  throw new Error(`transport demo: expected 3 free seats, got ${rideSim.freeSeats(truck)}`);
}
if (rideSim.vehicleOf(passenger)?.index !== truck.index) {
  throw new Error("transport demo: vehicleOf does not point back at the truck");
}
if (rideSim.passengers(truck).length !== 1) {
  throw new Error("transport demo: the truck should carry exactly one unit");
}

for (let i = 0; i < 60; i++) {
  rideSim.tick(16);
}

const rideRows = JSON.parse(rideSim.getWorldAsJson()).entities;
const rowFor = (id) =>
  rideRows.find((e) => e.id?.index === id.index && e.id?.generation === id.generation);
const truckRow = rowFor(truck);
const passengerRow = rowFor(passenger);
if (truckRow.position.x === 10.0) {
  throw new Error("transport demo: the truck never drove off");
}
if (
  passengerRow.position.x !== truckRow.position.x ||
  passengerRow.position.y !== truckRow.position.y
) {
  throw new Error(
    `transport demo: a passenger belongs where its vehicle is, got ${JSON.stringify(passengerRow.position)}`,
  );
}

rideSim.unboard(passenger);
if (rideSim.freeSeats(truck) !== 4 || rideSim.vehicleOf(passenger) !== null) {
  throw new Error("transport demo: unboard left the seat taken");
}

console.log("wasm transport demo ok");
