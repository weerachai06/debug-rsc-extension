import { createRoot } from 'react-dom/client';
import { type ChunkSource, networkSource, pageSource } from '../lib/capture';
import { loadFlight } from '../lib/flight';
import { PORT_NAME } from '../lib/protocol';
import wasmUrl from '../wasm/flight_bg.wasm?url';
import { App } from './App';
import './style.css';

const { tabId } = chrome.devtools.inspectedWindow;
const { network } = chrome.devtools;

const sources = [
  pageSource({ connect: () => chrome.tabs.connect(tabId, { name: PORT_NAME }), onNavigated: network.onNavigated }),
  networkSource(network),
];
const source: ChunkSource = (emit) => {
  const stops = sources.map((start) => start(emit));
  return () => stops.forEach((stop) => stop());
};

loadFlight(wasmUrl).then((create) =>
  createRoot(document.getElementById('app')!).render(<App create={create} source={source} />),
);
