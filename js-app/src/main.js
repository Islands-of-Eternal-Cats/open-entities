import init, { JsPosition, JsVelocity, move_position } from "open-entities-wasm";
let entities = []; let initialized = false;
const statusEl = document.getElementById("status");
const entityListEl = document.getElementById("entity-list");
const addEntityBtn = document.getElementById("add-entity");
const moveAllBtn = document.getElementById("move-all");
async function initWasm() {
  try {
    await init();
    initialized = true;
    statusEl.textContent = "WASM loaded successfully!";
    renderEntities();
    createEntity();
    setInterval(moveAllEntities, 1000);
  } catch (error) {
    statusEl.textContent = `Error loading WASM: ${error.message}`;
    console.error("WASM init error:", error);
  }
}
function createEntity(x = Math.random() * 100, y = Math.random() * 100) { const velocity = new JsVelocity(Math.random() * 2 - 1, Math.random() * 2 - 1); const position = new JsPosition(x, y); const entity = { id: entities.length, position, velocity }; entities.push(entity); renderEntities(); return entity; }
function moveAllEntities() {
  if (!initialized) return;
  entities.forEach(entity => {
    entity.position = move_position(entity.position, entity.velocity);
  });
  renderEntities();
}
function renderEntities() { if (!initialized) return; entityListEl.innerHTML = entities.map(entity => { const pos = entity.position; const vel = entity.velocity; return `<div class="entity"><strong>Entity ${entity.id}</strong><br>Position: (${pos.x().toFixed(2)}, ${pos.y().toFixed(2)})<br>Velocity: (${vel.vx().toFixed(2)}, ${vel.vy().toFixed(2)})</div>`; }).join(""); }
addEntityBtn.addEventListener("click", () => { createEntity(); });
moveAllBtn.addEventListener("click", () => { moveAllEntities(); });
initWasm();
