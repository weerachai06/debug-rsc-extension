import { expect, test } from 'vitest';
import { CHUNK_EVENT, PORT_NAME } from './protocol';
import { installRelay, type RelayPort } from './relay';

const setup = () => {
  const events = new EventTarget();
  const connectListeners: ((port: RelayPort) => void)[] = [];
  installRelay(events, { onConnect: { addListener: (listener) => void connectListeners.push(listener) } });

  const hookEmits = (json: string) => events.dispatchEvent(new CustomEvent(CHUNK_EVENT, { detail: json }));
  const connect = (name = PORT_NAME) => {
    const received: string[] = [];
    const disconnectListeners: (() => void)[] = [];
    const port: RelayPort = {
      name,
      postMessage: (message) => void received.push(message),
      onDisconnect: { addListener: (listener) => void disconnectListeners.push(listener) },
    };
    connectListeners.forEach((listener) => listener(port));
    return { received, disconnect: () => disconnectListeners.forEach((listener) => listener()) };
  };
  return { hookEmits, connect };
};

test('replays what was captured before the panel connected, then streams live', () => {
  const { hookEmits, connect } = setup();
  hookEmits('a');
  hookEmits('b');

  const panel = connect();
  hookEmits('c');

  expect(panel.received).toEqual(['a', 'b', 'c']);
});

test('ignores ports that are not the panel', () => {
  const { hookEmits, connect } = setup();
  hookEmits('a');

  expect(connect('someone-else').received).toEqual([]);
});

test('stops posting to a disconnected port', () => {
  const { hookEmits, connect } = setup();
  const panel = connect();
  hookEmits('a');
  panel.disconnect();
  hookEmits('b');

  expect(panel.received).toEqual(['a']);
});
