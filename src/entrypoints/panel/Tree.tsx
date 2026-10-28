import { useState } from 'preact/hooks';
import type { Loc } from '../../bindings/Loc';
import type { Flight } from '../../lib/flight';
import { formatPath } from './path';

type Props = {
  flight: Flight;
  at: Loc;
  label: string;
  preview: string;
  expandable: boolean;
  /** Where this node points, when it is a reference that resolved. */
  target?: Loc | null;
  /** Makes `to` the root of the detail pane. `from` is the reference that was clicked. */
  onJump: (to: Loc, from: Loc) => void;
  defaultOpen?: boolean;
};

/** One value of the resolved tree. Children are asked of the session on every render, so late-arriving rows show up. */
export const Branch = ({ flight, at, label, preview, expandable, target, onJump, defaultOpen = false }: Props) => {
  const [open, setOpen] = useState(defaultOpen);
  const path = formatPath(at);
  return (
    <li>
      {expandable ? (
        <button class="toggle" onClick={() => setOpen(!open)}>
          {open ? '▾' : '▸'}
        </button>
      ) : (
        <span class="toggle" />
      )}
      <span class="label">{label}</span>{' '}
      {target ? (
        <button class="preview jump" title={`Go to ${formatPath(target)}`} onClick={() => onJump(target, at)}>
          {preview} →
        </button>
      ) : (
        <span class="preview">{preview}</span>
      )}{' '}
      <button class="copy" title={path} onClick={() => navigator.clipboard.writeText(path)}>
        copy path
      </button>
      {open && (
        <ul>
          {flight.children(at).map((node) => (
            <Branch
              key={formatPath(node.at)}
              flight={flight}
              at={node.at}
              label={String(node.key ?? '')}
              preview={`${node.kind} ${node.preview}`}
              expandable={node.expandable}
              target={node.target}
              onJump={onJump}
            />
          ))}
        </ul>
      )}
    </li>
  );
};
