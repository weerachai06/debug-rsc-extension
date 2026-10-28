import { readFileSync } from 'node:fs';
import { beforeAll, expect, test } from 'vitest';
import type { Chunk } from '../bindings/Chunk';
import { type PagePort, pageSource } from './capture';
import { type Flight, loadFlight } from './flight';
import { type HookWindow, installHook } from './hook';
import { CHUNK_EVENT, PORT_NAME } from './protocol';
import { installRelay, type RelayPort, type RelayRuntime } from './relay';

let create: () => Flight;
beforeAll(async () => {
  create = await loadFlight(readFileSync('src/wasm/flight_bg.wasm'));
});

const document_ = JSON.parse(readFileSync('fixtures/next-16-prod-document.json', 'utf8')) as Chunk[];

// What Next's inline scripts push for that document: [0], then [1, text] per script.
const segments = [[0], ...document_.flatMap((c) => (c.body.kind === 'text' ? [[1, c.body.data]] : []))];

/** A runtime port pair. Like Chrome's, delivery is asynchronous, so listeners attached after connect() still hear the replay. */
const channel = () => {
  const connectListeners: ((port: RelayPort) => void)[] = [];
  const runtime: RelayRuntime = { onConnect: { addListener: (listener) => void connectListeners.push(listener) } };
  const connect = (): PagePort => {
    const messageListeners: ((message: string) => void)[] = [];
    const relaySide: RelayPort = {
      name: PORT_NAME,
      postMessage: (message) => queueMicrotask(() => messageListeners.forEach((listener) => listener(message))),
      onDisconnect: { addListener: () => {} },
    };
    connectListeners.forEach((listener) => listener(relaySide));
    return { onMessage: { addListener: (listener) => void messageListeners.push(listener) }, disconnect: () => {} };
  };
  return { runtime, connect };
};

const captureThroughTheExtension = async (panelOpenBeforeLoad: boolean) => {
  const events = new EventTarget();
  const { runtime, connect } = channel();
  installRelay(events, runtime);

  const domContentLoaded: (() => void)[] = [];
  const win: HookWindow & { __next_f?: unknown[] } = {
    location: { href: 'http://localhost:3999/' },
    document: { addEventListener: (_type, listener) => void domContentLoaded.push(listener) },
  };
  installHook(win, (chunk) => events.dispatchEvent(new CustomEvent(CHUNK_EVENT, { detail: JSON.stringify(chunk) })), 'doc:1');

  const flight = create();
  const openPanel = () => pageSource({ connect, onNavigated: { addListener: () => {}, removeListener: () => {} } })(flight.feed);

  if (panelOpenBeforeLoad) openPanel();
  segments.forEach((segment) => (win.__next_f = win.__next_f || []).push(segment));
  domContentLoaded.forEach((listener) => listener());
  if (!panelOpenBeforeLoad) openPanel();

  await new Promise((resolve) => setTimeout(resolve));
  const [stream] = flight.overview().streams;
  flight.dispose();
  return stream;
};

const direct = () => {
  const flight = create();
  document_.forEach(flight.feed);
  const [stream] = flight.overview().streams;
  flight.dispose();
  return stream;
};

test.each([
  ['open while the page loads', true],
  ['opened after the page loaded (replay)', false],
])('DevTools %s sees exactly what feeding the fixture directly gives', async (_name, openFirst) => {
  const viaExtension = await captureThroughTheExtension(openFirst);

  expect(viaExtension?.diagnostics).toEqual([]);
  expect(viaExtension?.state).toBe('ended');
  expect(viaExtension?.rows.length).toBeGreaterThan(10);
  expect(viaExtension?.rows).toEqual(direct()?.rows);
  expect(viaExtension?.bytes).toBe(direct()?.bytes);
});
