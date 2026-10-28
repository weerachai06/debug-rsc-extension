import type { Chunk } from '../bindings/Chunk';

export type Emit = (chunk: Chunk) => void;

/** Replays what was captured so far, then streams live. Returns stop. */
export type ChunkSource = (emit: Emit) => () => void;

export type PagePort = {
  onMessage: { addListener(listener: (message: string) => void): void };
  disconnect(): void;
};

type Listeners<T extends (...args: never[]) => void> = {
  addListener(listener: T): void;
  removeListener(listener: T): void;
};

/** The page's relay: replays the current document, then streams its live pushes. */
export const pageSource =
  ({ connect, onNavigated }: { connect: () => PagePort; onNavigated: Listeners<() => void> }): ChunkSource =>
  (emit) => {
    let port: PagePort | undefined;
    const attach = () => {
      port?.disconnect();
      port = connect();
      port.onMessage.addListener((message) => emit(JSON.parse(message) as Chunk));
    };
    attach();
    onNavigated.addListener(attach);
    return () => {
      onNavigated.removeListener(attach);
      port?.disconnect();
    };
  };

export type NetworkEntry = {
  startedDateTime: string;
  request: { url: string; method: string };
  response: { status: number; content: { mimeType: string } };
  getContent(callback: (content: string, encoding: string) => void): void;
};

export type NetworkApi = {
  onRequestFinished: Listeners<(entry: NetworkEntry) => void>;
  getHAR(callback: (log: { entries: NetworkEntry[] }) => void): void;
};

/** Finished RSC fetches (navigations, prefetches, server actions). Only sees traffic while DevTools is open. */
export const networkSource =
  (network: NetworkApi): ChunkSource =>
  (emit) => {
    const seen = new Set<string>();
    const handle = (entry: NetworkEntry) => {
      if (!entry.response.content.mimeType.startsWith('text/x-component')) return;
      const { url, method } = entry.request;
      const stream = `fetch:${entry.startedDateTime}:${method}:${url}`;
      if (seen.has(stream)) return;
      seen.add(stream);
      entry.getContent((content, encoding) => {
        const data = content ?? '';
        const status = entry.response.status;
        emit({ stream, body: { kind: 'open', origin: { kind: 'fetch', url, method, status } } });
        emit({ stream, body: encoding === 'base64' ? { kind: 'base64', data } : { kind: 'text', data } });
        emit({ stream, body: { kind: 'end' } });
      });
    };
    // Subscribe before reading the HAR so nothing finishing in between is lost; `seen` absorbs the overlap.
    network.onRequestFinished.addListener(handle);
    network.getHAR((log) => log.entries.forEach(handle));
    return () => network.onRequestFinished.removeListener(handle);
  };
