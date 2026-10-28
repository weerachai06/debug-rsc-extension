import type { Body } from '../bindings/Body';
import type { Emit } from './capture';

export type HookWindow = {
  location: { href: string };
  document: { addEventListener(type: 'DOMContentLoaded', listener: () => void): void };
};

type Push = (...segments: unknown[]) => unknown;

// Next's segments: [0] bootstrap, [1, text], [2, formState], [3, base64]. [0] only opens the stream.
const toBody = (segment: unknown): Body | undefined => {
  const [kind, value] = Array.isArray(segment) ? segment : [];
  if (kind === 0) return undefined;
  if (kind === 1 && typeof value === 'string') return { kind: 'text', data: value };
  if (kind === 3 && typeof value === 'string') return { kind: 'base64', data: value };
  if (kind === 2) return { kind: 'formState', json: JSON.stringify(value) ?? 'null' };
  return { kind: 'other', json: JSON.stringify(segment) ?? 'null' };
};

/**
 * Records everything pushed to `self.__next_f` without changing what Next sees.
 * Must run before Next's inline bootstrap. Nothing is emitted on pages that never use `__next_f`.
 */
export const installHook = (win: HookWindow, emit: Emit, stream: string): void => {
  let opened = false;

  // Recording must never break the page, so nothing here is allowed to throw.
  const record = (segment: unknown) => {
    try {
      if (!opened) {
        opened = true;
        emit({ stream, body: { kind: 'open', origin: { kind: 'document', url: win.location.href } } });
      }
      const body = toBody(segment);
      if (body) emit({ stream, body });
    } catch {}
  };

  // Next later does `array.push = nextServerDataCallback`; the setter keeps that as the inner push.
  const instrument = (array: unknown[]) => {
    let inner: Push = Array.prototype.push;
    const recording: Push = (...segments) => {
      segments.forEach((segment) => record(segment));
      return inner.apply(array, segments);
    };
    Object.defineProperty(array, 'push', {
      configurable: true,
      get: () => recording,
      set: (next: Push) => {
        inner = next;
      },
    });
  };

  // Every Next inline script re-assigns `self.__next_f = self.__next_f || []`, so the same array arrives repeatedly.
  let value: unknown;
  Object.defineProperty(win, '__next_f', {
    configurable: true,
    get: () => value,
    set: (next: unknown) => {
      if (next === value) return;
      if (Array.isArray(next)) instrument(next);
      value = next;
    },
  });

  win.document.addEventListener('DOMContentLoaded', () => {
    if (!opened) return;
    try {
      emit({ stream, body: { kind: 'end' } });
    } catch {}
  });
};
