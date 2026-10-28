import { CHUNK_EVENT, PORT_NAME } from './protocol';

export type RelayPort = {
  name: string;
  postMessage(message: string): void;
  onDisconnect: { addListener(listener: () => void): void };
};

export type RelayRuntime = {
  onConnect: { addListener(listener: (port: RelayPort) => void): void };
};

/** Buffers the hook's chunks for the whole page life, and replays them to each panel that connects. */
export const installRelay = (events: EventTarget, runtime: RelayRuntime): void => {
  const buffer: string[] = [];
  const ports = new Set<RelayPort>();

  events.addEventListener(CHUNK_EVENT, (event) => {
    const json = (event as CustomEvent).detail;
    if (typeof json !== 'string') return;
    buffer.push(json);
    ports.forEach((port) => port.postMessage(json));
  });

  runtime.onConnect.addListener((port) => {
    if (port.name !== PORT_NAME) return;
    buffer.forEach((json) => port.postMessage(json));
    ports.add(port);
    port.onDisconnect.addListener(() => ports.delete(port));
  });
};
