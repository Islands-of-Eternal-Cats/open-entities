// `step()` at 100 000 movers under wasm32 in Node: the wasm half of the step 4 budget
// (≤ 15 ms per step; native is `cargo bench -p open_entities --bench step`).
//
//   wasm-pack build wasm-bindings --target nodejs && node wasm-bindings/demo/bench.mjs
//
// The world is the one `step_100k_moving` builds: the same templates, the same SplitMix64 seed,
// so the two numbers compare like for like. Timings are local only; CI does not run this.
import { Simulation } from "../pkg/open_entities_wasm.js";

const UNITS = 100_000;
const MAP = 2_000_000n; // 2000 × 2000 map units, in milli-units
const SEED = 0x5eed0f57e9n;
const WARMUP_STEPS = 50;
const MEASURED_STEPS = 200;

const TEMPLATES = `entities:
  mover:
    faction: 1
    velocity: { vx: 0.0, vy: 0.0 }
    base_move_speed: 5.0
`;

const MASK = (1n << 64n) - 1n;
let state = SEED;
function next() {
  state = (state + 0x9e3779b97f4a7c15n) & MASK;
  let z = state;
  z = ((z ^ (z >> 30n)) * 0xbf58476d1ce4e5b9n) & MASK;
  z = ((z ^ (z >> 27n)) * 0x94d049bb133111ebn) & MASK;
  return z ^ (z >> 31n);
}
/** A coordinate in map units; milli-units / 1000 converts back exactly. */
const coord = () => Number(next() % MAP) / 1000;

function median(sorted) {
  const mid = sorted.length >> 1;
  return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}

function measure(label, run) {
  for (let i = 0; i < WARMUP_STEPS; i++) run();
  const times = [];
  for (let i = 0; i < MEASURED_STEPS; i++) {
    const start = performance.now();
    run();
    times.push(performance.now() - start);
  }
  times.sort((a, b) => a - b);
  const mean = times.reduce((a, b) => a + b, 0) / times.length;
  const p95 = times[Math.floor(times.length * 0.95)];
  console.log(
    `${label}: median ${median(times).toFixed(2)} ms, mean ${mean.toFixed(2)} ms, ` +
      `p95 ${p95.toFixed(2)} ms (${MEASURED_STEPS} runs)`,
  );
}

const sim = new Simulation();
sim.loadTemplatesYaml(TEMPLATES);
for (let i = 0; i < UNITS; i++) {
  // Position first, then target: the same draw order as the native bench.
  const position = { x: coord(), y: coord() };
  const move_target = { x: coord(), y: coord() };
  sim.spawnEntity("mover", { position, move_target });
}

console.log(`wasm32 in Node ${process.version}, ${UNITS} movers`);
measure("step_100k_moving", () => sim.step());
if (typeof sim.writeFrame === "function") {
  measure("writeFrame_100k", () => sim.writeFrame());
}
