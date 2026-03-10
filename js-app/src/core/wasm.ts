/**
 * WASM core wrapper. Single place to init and re-export the game core API.
 * Visualization layer should depend only on this module and types from ./types.
 */
import initWasmModule, {
  JsPosition,
  JsVelocity,
  move_position,
} from "open-entities-wasm";

let initialized = false;

const WASM_URL = "/wasm_bindings_bg.wasm";

export async function initWasm(): Promise<void> {
  await initWasmModule(WASM_URL);
  initialized = true;
}

export function isWasmReady(): boolean {
  return initialized;
}

export { JsPosition, JsVelocity, move_position };
