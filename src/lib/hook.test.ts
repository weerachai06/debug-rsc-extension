import { expect, test } from 'vitest';
import type { Chunk } from '../bindings/Chunk';
import { type HookWindow, installHook } from './hook';

type NextWindow = HookWindow & { __next_f?: unknown[] };

const setup = (emit: (chunk: Chunk) => void) => {
  const listeners: (() => void)[] = [];
  const win: NextWindow = {
    location: { href: 'https://example.com/' },
    document: { addEventListener: (_type, listener) => void listeners.push(listener) },
  };
  installHook(win, emit, 'doc:1');
  // What each Next inline script does: (self.__next_f = self.__next_f || []).push(segment)
  const push = (segment: unknown) => (win.__next_f = win.__next_f || []).push(segment);
  const domContentLoaded = () => listeners.forEach((listener) => listener());
  return { win, push, domContentLoaded };
};

const doc = (body: Chunk['body']): Chunk => ({ stream: 'doc:1', body });
const open = doc({ kind: 'open', origin: { kind: 'document', url: 'https://example.com/' } });

test('records every segment kind and leaves the callback Next installs intact', () => {
  const emitted: Chunk[] = [];
  const { win, push, domContentLoaded } = setup((chunk) => emitted.push(chunk));

  push([0]);
  push([1, 'a']);
  push([3, 'aGk=']);
  push([2, ['x']]);
  push([9, 'odd']);

  // app-index.tsx: drain what is buffered, then take over push
  const seen: unknown[] = [];
  const nextServerDataCallback = (segment: unknown) => seen.push(segment);
  win.__next_f!.forEach((segment) => nextServerDataCallback(segment));
  win.__next_f!.push = nextServerDataCallback;

  push([1, 'b']);
  domContentLoaded();

  expect(emitted).toEqual([
    open,
    doc({ kind: 'text', data: 'a' }),
    doc({ kind: 'base64', data: 'aGk=' }),
    doc({ kind: 'formState', json: '["x"]' }),
    doc({ kind: 'other', json: '[9,"odd"]' }),
    doc({ kind: 'text', data: 'b' }),
    doc({ kind: 'end' }),
  ]);
  expect(seen).toEqual([[0], [1, 'a'], [3, 'aGk='], [2, ['x']], [9, 'odd'], [1, 'b']]);
});

test('opens the stream even when the first segment is not the bootstrap one', () => {
  const emitted: Chunk[] = [];
  const { push } = setup((chunk) => emitted.push(chunk));

  push([1, 'a']);

  expect(emitted).toEqual([open, doc({ kind: 'text', data: 'a' })]);
});

test('emits nothing on pages that never use __next_f', () => {
  const emitted: Chunk[] = [];
  const { domContentLoaded } = setup((chunk) => emitted.push(chunk));

  domContentLoaded();

  expect(emitted).toEqual([]);
});

test('Next still receives every segment when emit throws', () => {
  const { win, push } = setup(() => {
    throw new Error('boom');
  });
  const seen: unknown[] = [];
  push([0]);
  win.__next_f!.push = (segment: unknown) => seen.push(segment);

  push([1, 'a']);

  expect(seen).toEqual([[1, 'a']]);
});
