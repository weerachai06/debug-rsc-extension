import type { Chunk } from '../bindings/Chunk';
import type { Hit } from '../bindings/Hit';
import type { Loc } from '../bindings/Loc';
import type { Node } from '../bindings/Node';
import type { Overview } from '../bindings/Overview';
import type { Query } from '../bindings/Query';
import init, { type InitInput, Flight as WasmFlight } from '../wasm/flight';

export type Flight = {
  feed(chunk: Chunk): void;
  overview(): Overview;
  children(at: Loc): Node[];
  search(query: Query): Hit[];
  dispose(): void;
};

const wrap = (session: WasmFlight): Flight => ({
  feed: (chunk) => session.feed(JSON.stringify(chunk)),
  overview: () => JSON.parse(session.overview()) as Overview,
  children: (at) => JSON.parse(session.children(JSON.stringify(at))) as Node[],
  search: (query) => JSON.parse(session.search(JSON.stringify(query))) as Hit[],
  dispose: () => session.free(),
});

/** Loads the WASM once; each call of the returned factory is an independent session. */
export const loadFlight = async (wasm: InitInput): Promise<() => Flight> => {
  await init({ module_or_path: wasm });
  return () => wrap(new WasmFlight());
};
