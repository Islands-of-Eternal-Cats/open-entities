/**
 * DOM writes that touch the document only when the value actually changed.
 *
 * The HUD is synced after every core tick, and a write of the same string is not free: it
 * replaces the text node, which drops the player's text selection, the caret, and any
 * in-progress drag. Compare first, write second.
 */

/** Sets `textContent` unless the element already shows exactly that text. */
export function setText(el: Element, text: string): void {
  if (el.textContent !== text) el.textContent = text;
}

/**
 * Sets `innerHTML` unless the same markup was written by this function last time.
 *
 * The comparison is against what was last written, not against the serialized DOM: the
 * browser normalizes markup on parse, so reading `innerHTML` back rarely matches the source.
 */
export function setHtml(el: Element, html: string): void {
  const last = lastHtml.get(el);
  if (last === html) return;
  lastHtml.set(el, html);
  el.innerHTML = html;
}

const lastHtml = new WeakMap<Element, string>();
