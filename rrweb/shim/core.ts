import { call, on } from './transport';
export const invoke = <T>(cmd: string, args?: Record<string, unknown>) => call<T>(cmd, args ?? {});
export const convertFileSrc = (path: string) => `/files?path=${encodeURIComponent(path)}`;
export const isTauri = () => false;

// Tauri Channel over the relay: the argument travels as { __rrChannel: id }, the bridge (bridge/bridge.ts) creates a
// real Channel for it and sends each message back as the event "__rrch:<id>" (binary data base64-encoded).
let channelSeq = 0;
const fromWire = (p: unknown): unknown => {
  if (p && typeof p === 'object' && typeof (p as { __rrb64?: unknown }).__rrb64 === 'string') {
    const s = atob((p as { __rrb64: string }).__rrb64);
    const bytes = new Uint8Array(s.length);
    for (let i = 0; i < s.length; i++) bytes[i] = s.charCodeAt(i);
    return bytes.buffer;
  }
  return p;
};
export class Channel<T = unknown> {
  readonly id = `${Date.now().toString(36)}-${++channelSeq}`;
  #onmessage: (response: T) => void = () => {};
  constructor(onmessage?: (response: T) => void) {
    if (onmessage) this.#onmessage = onmessage;
    on(`__rrch:${this.id}`, (payload) => this.#onmessage(fromWire(payload) as T));
  }
  set onmessage(handler: (response: T) => void) { this.#onmessage = handler; }
  get onmessage() { return this.#onmessage; }
  toJSON() { return { __rrChannel: this.id }; }
}
