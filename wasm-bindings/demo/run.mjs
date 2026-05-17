import { Simulation } from "../pkg/open_entities_wasm.js";

const sim = new Simulation();
console.log(sim.hello());
const json = sim.world_json();
console.log(json);
const parsed = JSON.parse(json);
if (parsed.version !== 3 || !Array.isArray(parsed.entities) || parsed.entities.length !== 0) {
  throw new Error(`unexpected empty world JSON: ${json}`);
}
