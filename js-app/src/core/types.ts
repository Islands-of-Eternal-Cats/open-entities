/**
 * App-level types that combine WASM types with visualization/UI state.
 */
import type { JsPosition, JsVelocity } from "./wasm";

/** Entity as seen by the app: id + WASM position/velocity. */
export interface GameEntity {
  id: number;
  position: JsPosition;
  velocity: JsVelocity;
}
