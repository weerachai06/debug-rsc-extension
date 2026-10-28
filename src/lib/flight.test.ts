import { readFileSync } from 'node:fs';
import { beforeAll, expect, test } from 'vitest';
import type { Chunk } from '../bindings/Chunk';
import type { ChunkSource } from './capture';
import { type Flight, loadFlight } from './flight';

const fixtureSource =
  (chunks: Chunk[]): ChunkSource =>
  (emit) => {
    chunks.forEach(emit);
    return () => {};
  };

const fixture = (name: string) => JSON.parse(readFileSync(`fixtures/${name}.json`, 'utf8')) as Chunk[];

let create: () => Flight;
beforeAll(async () => {
  create = await loadFlight(readFileSync('src/wasm/flight_bg.wasm'));
});

test('a fixture streamed through the real WASM session shows up in overview', () => {
  const flight = create();

  fixtureSource(fixture('minimal'))(flight.feed);

  expect(flight.overview().streams).toMatchObject([
    { id: 'doc:1', origin: { kind: 'document' }, state: 'ended', bytes: 22, diagnostics: [], rows: [{ id: 0, tag: { kind: 'model' } }] },
  ]);
  flight.dispose();
});

test('a real Next payload crosses the JSON seam with the shapes the bindings promise', () => {
  const flight = create();
  fixtureSource(fixture('next-16-prod-fetch'))(flight.feed);

  const [stream] = flight.overview().streams;
  expect(stream?.diagnostics).toEqual([]);

  // Option is null, never undefined; Seg is a string or a number.
  const page = stream?.rows.find((row) => row.id === 7);
  const [main] = flight.children(page!.at);
  expect(main).toMatchObject({ key: 'type', kind: 'scalar', target: null, expandable: false });

  const [hit] = flight.search({ text: 'countDown', field: 'key', exact: true, scope: null, limit: 5 });
  expect(hit).toMatchObject({ key: 'countDown', preview: '1793465999', at: { id: 7 } });
  expect(hit?.at.path).toEqual(['props', 'children', 1, 'props', 'filterData', 'countDown']);

  // Walk from the hit's parent to its sibling through the same locations the panel uses.
  const filterData = { ...hit!.at, path: hit!.at.path.slice(0, -1) };
  expect(flight.children(filterData).map((node) => node.key)).toEqual(['brands', 'countDown']);
  flight.dispose();
});

test('a real payload fed in odd pieces gives the same overview as fed whole', () => {
  const text = fixture('next-16-prod-fetch')
    .map((chunk) => (chunk.body.kind === 'text' ? chunk.body.data : ''))
    .join('');
  const feedAll = (pieces: string[]) => {
    const flight = create();
    flight.feed({ stream: 's', body: { kind: 'open', origin: { kind: 'document', url: 'u' } } });
    pieces.forEach((data) => flight.feed({ stream: 's', body: { kind: 'text', data } }));
    flight.feed({ stream: 's', body: { kind: 'end' } });
    const overview = flight.overview();
    flight.dispose();
    return overview;
  };

  // Text chunks always split on characters, so every piece is valid on its own.
  const pieces = Array.from(text.matchAll(/[^]{1,97}/gu), (m) => m[0]);
  expect(feedAll(pieces)).toEqual(feedAll([text]));
});
