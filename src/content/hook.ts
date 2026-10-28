import { installHook } from '../lib/hook';
import { CHUNK_EVENT } from '../lib/protocol';

installHook(
  window,
  (chunk) => window.dispatchEvent(new CustomEvent(CHUNK_EVENT, { detail: JSON.stringify(chunk) })),
  `doc:${performance.timeOrigin}`,
);
