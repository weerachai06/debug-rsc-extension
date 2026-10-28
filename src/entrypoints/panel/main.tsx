import { render } from 'preact';
import { browser } from 'wxt/browser';
import { type ChunkSource, networkSource, pageSource } from '../../lib/capture';
import { loadFlight } from '../../lib/flight';
import { PORT_NAME } from '../../lib/protocol';
import wasmUrl from '../../wasm/flight_bg.wasm?url';
import { App } from './App';
import './style.css';

const { tabId } = browser.devtools.inspectedWindow;
const { network } = browser.devtools;

const sources = [
  pageSource({ connect: () => browser.tabs.connect(tabId, { name: PORT_NAME }), onNavigated: network.onNavigated }),
  networkSource(network),
];
const source: ChunkSource = (emit) => {
  const stops = sources.map((start) => start(emit));
  return () => stops.forEach((stop) => stop());
};

loadFlight(wasmUrl).then((create) => render(<App create={create} source={source} />, document.getElementById('app')!));
