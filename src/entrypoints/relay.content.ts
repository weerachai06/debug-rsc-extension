import { browser } from 'wxt/browser';
import { installRelay } from '../lib/relay';

export default defineContentScript({
  matches: ['http://*/*', 'https://*/*'],
  runAt: 'document_start',
  main() {
    installRelay(window, browser.runtime);
  },
});
