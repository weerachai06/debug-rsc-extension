import { expect, test } from 'vitest';
import type { Chunk } from '../bindings/Chunk';
import { type NetworkEntry, networkSource, type PagePort, pageSource } from './capture';

const collect = () => {
  const chunks: Chunk[] = [];
  return { chunks, emit: (chunk: Chunk) => void chunks.push(chunk) };
};

const chunk: Chunk = { stream: 'doc:1', body: { kind: 'text', data: 'a' } };

const fakePage = () => {
  const ports: { port: PagePort; deliver: (message: string) => void; disconnected: () => boolean }[] = [];
  const navigated: (() => void)[] = [];
  const source = pageSource({
    connect: () => {
      let messageListener: (message: string) => void = () => {};
      let disconnected = false;
      const port: PagePort = {
        onMessage: { addListener: (listener) => void (messageListener = listener) },
        disconnect: () => void (disconnected = true),
      };
      ports.push({ port, deliver: (message) => messageListener(message), disconnected: () => disconnected });
      return port;
    },
    onNavigated: {
      addListener: (listener) => void navigated.push(listener),
      removeListener: (listener) => void navigated.splice(navigated.indexOf(listener), 1),
    },
  });
  return { source, ports, navigated };
};

test('pageSource emits the chunks the relay posts', () => {
  const { source, ports } = fakePage();
  const { chunks, emit } = collect();

  source(emit);
  ports[0]?.deliver(JSON.stringify(chunk));

  expect(chunks).toEqual([chunk]);
});

test('pageSource reconnects after a navigation, so the new document is replayed', () => {
  const { source, ports, navigated } = fakePage();
  const { chunks, emit } = collect();

  source(emit);
  navigated.forEach((listener) => listener());
  ports[1]?.deliver(JSON.stringify(chunk));

  expect(ports).toHaveLength(2);
  expect(ports[0]?.disconnected()).toBe(true);
  expect(chunks).toEqual([chunk]);
});

test('stopping pageSource disconnects and stops listening for navigations', () => {
  const { source, ports, navigated } = fakePage();

  source(() => {})();

  expect(ports[0]?.disconnected()).toBe(true);
  expect(navigated).toHaveLength(0);
});

const entry = (over: Partial<{ url: string; mimeType: string; content: string; encoding: string; started: string }> = {}): NetworkEntry => {
  const { url = 'https://example.com/?_rsc=x', mimeType = 'text/x-component', content = '0:null\n', encoding = '', started = 't1' } = over;
  return {
    startedDateTime: started,
    request: { url, method: 'GET' },
    response: { status: 200, content: { mimeType } },
    getContent: (callback) => callback(content, encoding),
  };
};

const fakeNetwork = (har: NetworkEntry[] = []) => {
  const listeners: ((entry: NetworkEntry) => void)[] = [];
  const network = {
    onRequestFinished: {
      addListener: (listener: (entry: NetworkEntry) => void) => void listeners.push(listener),
      removeListener: (listener: (entry: NetworkEntry) => void) => void listeners.splice(listeners.indexOf(listener), 1),
    },
    getHAR: (callback: (log: { entries: NetworkEntry[] }) => void) => callback({ entries: har }),
  };
  return { network, finish: (e: NetworkEntry) => listeners.forEach((listener) => listener(e)), listeners };
};

const fetchStream = 'fetch:t1:GET:https://example.com/?_rsc=x';

test('networkSource turns a finished RSC response into Open, Text, End', () => {
  const { network, finish } = fakeNetwork();
  const { chunks, emit } = collect();

  networkSource(network)(emit);
  finish(entry());

  expect(chunks).toEqual([
    { stream: fetchStream, body: { kind: 'open', origin: { kind: 'fetch', url: 'https://example.com/?_rsc=x', method: 'GET', status: 200 } } },
    { stream: fetchStream, body: { kind: 'text', data: '0:null\n' } },
    { stream: fetchStream, body: { kind: 'end' } },
  ]);
});

test('networkSource keeps base64 bodies as Base64', () => {
  const { network, finish } = fakeNetwork();
  const { chunks, emit } = collect();

  networkSource(network)(emit);
  finish(entry({ content: 'aGk=', encoding: 'base64' }));

  expect(chunks[1]?.body).toEqual({ kind: 'base64', data: 'aGk=' });
});

test('networkSource ignores responses that are not RSC', () => {
  const { network, finish } = fakeNetwork();
  const { chunks, emit } = collect();

  networkSource(network)(emit);
  finish(entry({ mimeType: 'text/html' }));

  expect(chunks).toEqual([]);
});

test('networkSource backfills from the HAR and does not repeat a request seen both ways', () => {
  const earlier = entry({ started: 't0', url: 'https://example.com/?_rsc=old' });
  const { network, finish } = fakeNetwork([earlier, entry()]);
  const { chunks, emit } = collect();

  networkSource(network)(emit);
  finish(entry());

  expect(chunks.filter((c) => c.body.kind === 'open').map((c) => c.stream)).toEqual([
    'fetch:t0:GET:https://example.com/?_rsc=old',
    fetchStream,
  ]);
});

test('stopping networkSource stops listening', () => {
  const { network, listeners } = fakeNetwork();

  networkSource(network)(() => {})();

  expect(listeners).toHaveLength(0);
});
