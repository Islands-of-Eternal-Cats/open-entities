/**
 * ECS web worker: loads WASM and runs the simulation.
 * Listens for init/snapshot/frame/submit/stress messages; posts back
 * ready/snapshot/frame/submitted/stressed/error.
 *
 * Orders reach the core only as commands (`Simulation.submit`): each applies at the start of the
 * next tick, and its outcome rides back on the `frame` that ran that tick. Nothing here changes
 * the world between steps.
 *
 * The world crosses the boundary as position frames (`Simulation.writeFrame()`, an `Int32Array`
 * transferred with the message) plus a metadata delta only when something changed; what each
 * message holds is decided in `worker-core.ts`. `getWorldAsJson()` is not on this path.
 */
import initWasmModule, { Simulation, tickMs } from "open_entities_wasm";
import { TICK_MS } from "./fixed-step";
import { WorkerCore, type Outgoing } from "./worker-core";
import type { WorkerInMessage, WorkerOutMessage } from "./worker-types";

let core: WorkerCore | null = null;

function post(msg: WorkerOutMessage): void {
  self.postMessage(msg);
}

function send({ message, transfer }: Outgoing): void {
  self.postMessage(message, { transfer });
}

self.onmessage = async (event: MessageEvent<WorkerInMessage>) => {
  const msg = event.data;
  try {
    if (msg.type === "init") {
      await initWasmModule({ module_or_path: msg.wasmBuffer });
      try {
        if (tickMs() !== TICK_MS) {
          throw new Error(
            `core tick is ${tickMs()} ms but the host clock uses ${TICK_MS} ms`
          );
        }
        const simulation = new Simulation();
        simulation.loadTemplatesYaml(msg.templatesYaml);
        simulation.loadMapYaml(msg.mapYaml);
        core = new WorkerCore(simulation);
      } catch (e) {
        post({
          type: "error",
          message: e instanceof Error ? e.message : String(e),
        });
        return;
      }
      post({ type: "ready" });
      return;
    }

    if (!core) {
      post({ type: "error", message: "Worker not initialized" });
      return;
    }

    if (msg.type === "frame") send(core.frame(msg.elapsedMs));
    else if (msg.type === "snapshot") send(core.snapshot());
    else if (msg.type === "submit") send(core.submit(msg.commands));
    else if (msg.type === "stress")
      send(core.stress(msg.count, msg.seed, msg.worldSize));
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    post({ type: "error", message });
  }
};
