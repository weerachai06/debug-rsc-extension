import { useEffect, useRef, useState } from 'preact/hooks';
import type { Chunk } from '../../bindings/Chunk';
import type { Overview } from '../../bindings/Overview';
import type { ChunkSource } from '../../lib/capture';
import type { Flight } from '../../lib/flight';

/** Feeds a source into a Flight session and re-reads its overview at most once per animation frame. */
export const useCapture = (create: () => Flight, source: ChunkSource) => {
  const [first] = useState(create);
  const flightRef = useRef(first);
  const chunksRef = useRef<Chunk[]>([]);
  const [overview, setOverview] = useState<Overview>(() => first.overview());

  useEffect(() => {
    let frame = 0;
    const stop = source((chunk) => {
      chunksRef.current.push(chunk);
      flightRef.current.feed(chunk);
      frame ||= requestAnimationFrame(() => {
        frame = 0;
        setOverview(flightRef.current.overview());
      });
    });
    return () => {
      stop();
      cancelAnimationFrame(frame);
    };
  }, [source]);

  useEffect(() => () => flightRef.current.dispose(), []);

  // Like the Network panel's Clear: the current document is not replayed.
  const clear = () => {
    flightRef.current.dispose();
    flightRef.current = create();
    chunksRef.current = [];
    setOverview(flightRef.current.overview());
  };

  const exportChunks = () => {
    const url = URL.createObjectURL(new Blob([JSON.stringify(chunksRef.current, null, 2)], { type: 'application/json' }));
    Object.assign(document.createElement('a'), { href: url, download: 'chunks.json' }).click();
    URL.revokeObjectURL(url);
  };

  return { flight: flightRef.current, overview, clear, exportChunks };
};
