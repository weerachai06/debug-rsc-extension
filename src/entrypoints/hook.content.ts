import { installHook } from '../lib/hook';
import { CHUNK_EVENT } from '../lib/protocol';

export default defineContentScript({
  matches: ['http://*/*', 'https://*/*'],
  runAt: 'document_start',
  world: 'MAIN',
  main() {
    installHook(
      window,
      (chunk) => window.dispatchEvent(new CustomEvent(CHUNK_EVENT, { detail: JSON.stringify(chunk) })),
      `doc:${performance.timeOrigin}`,
    );
  },
});
