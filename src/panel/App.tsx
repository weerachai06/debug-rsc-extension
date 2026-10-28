import { useState } from 'react';
import type { Loc } from '../bindings/Loc';
import type { RowTag } from '../bindings/RowTag';
import type { StreamSummary } from '../bindings/StreamSummary';
import type { ChunkSource } from '../lib/capture';
import type { Flight } from '../lib/flight';
import { formatPath } from './path';
import { Branch } from './Tree';
import { useCapture } from './useCapture';

type Selection = { at: Loc; label: string; preview: string };

const tagName = (tag: RowTag) => (tag.kind === 'unknown' ? `?${tag.ch}` : tag.kind);

const streamTitle = ({ origin, id }: StreamSummary) =>
  origin?.kind === 'fetch' ? `${origin.method} ${origin.url}` : (origin?.url ?? id);

export const App = ({ create, source }: { create: () => Flight; source: ChunkSource }) => {
  const { flight, overview, clear, exportChunks } = useCapture(create, source);
  const [query, setQuery] = useState('');
  const [selection, setSelection] = useState<Selection>();

  const hits = query ? flight.search({ text: query, field: 'any', exact: false, scope: null, limit: 200 }) : [];

  return (
    <>
      <header>
        <input
          type="search"
          placeholder="Search keys and values"
          value={query}
          onChange={(e) => setQuery(e.currentTarget.value)}
        />
        <button
          onClick={() => {
            clear();
            setSelection(undefined);
          }}
        >
          Clear
        </button>
        <button onClick={exportChunks}>Export chunks</button>
      </header>
      <main>
        <section>
          {query ? (
            <ul>
              {hits.map((hit) => (
                <li key={formatPath(hit.at)}>
                  <button
                    onClick={() => setSelection({ at: hit.at, label: formatPath(hit.at), preview: hit.preview })}
                  >
                    {formatPath(hit.at)} {hit.preview}
                  </button>
                </li>
              ))}
            </ul>
          ) : (
            overview.streams.map((stream) => (
              <div key={stream.id}>
                <h3>
                  {streamTitle(stream)} · {stream.state} · {stream.bytes} B
                </h3>
                {stream.diagnostics.map((d, i) => (
                  <p className="diagnostic" key={i}>
                    {d.code}: {d.message}
                  </p>
                ))}
                <ul>
                  {stream.rows.map((row) => (
                    <li key={row.id}>
                      <button
                        onClick={() =>
                          setSelection({
                            at: row.at,
                            label: row.id.toString(16),
                            preview: `${tagName(row.tag)} ${row.preview}`,
                          })
                        }
                      >
                        {row.id.toString(16)} {tagName(row.tag)} {row.bytes} B {row.preview}
                      </button>
                    </li>
                  ))}
                </ul>
              </div>
            ))
          )}
        </section>
        <section>
          {selection && (
            <ul className="tree">
              <Branch
                key={formatPath(selection.at)}
                flight={flight}
                expandable
                defaultOpen
                onJump={(to, from) => setSelection({ at: to, label: formatPath(to), preview: `← ${formatPath(from)}` })}
                {...selection}
              />
            </ul>
          )}
        </section>
      </main>
    </>
  );
};
