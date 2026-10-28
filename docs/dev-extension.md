# Running the extension as a dev extension

How to load RSC Debugger into Chrome as an unpacked extension, iterate on it, get a build from CI, and check that it works on a real Next.js app.

Chrome only. `world: 'MAIN'` content scripts and `chrome.devtools.panels` are used as Chrome implements them; Firefox and Safari have not been tried.

## 1. Build it and load it

Needs `fnm`, Rust with the `wasm32-unknown-unknown` target, and `wasm-pack` 0.13.1 (see the README).

```sh
fnm use
npm install
npm run build        # builds the WASM, then the extension -> .output/chrome-mv3
```

Then in Chrome:

1. Open `chrome://extensions`.
2. Turn on **Developer mode** (top right).
3. Click **Load unpacked** and choose the `.output/chrome-mv3` folder.

The extension has no toolbar icon and no popup. It adds an **RSC** tab to DevTools, which may sit under the `»` overflow menu.

**Always reload the inspected page** after loading or reloading the extension. Content scripts are only injected when a page loads, so a tab opened earlier has no hook.

## 2. Development loop

```sh
npm run dev          # builds the WASM and the extension, then rebuilds the panel and DevTools pages on change
```

- Output goes to `.output/chrome-mv3`, the same folder as a production build. After a rebuild, click the reload icon on the extension in `chrome://extensions`.
- There is no hot reload. Only the panel and DevTools pages are watched; after editing a content script (`src/content/`), run `npm run build:ext`.
- **Rust changes are not watched.** After editing `flight/`, run `npm run wasm`.
- **Changed a type in `flight/src/types.rs`?** Run `npm run bindings`, then commit the regenerated `src/bindings/`. CI fails if they are out of date.

## 3. Get a build without the toolchain

**Releases.** Every merge to `main` publishes a GitHub release `vX.Y.Z` with `rsc-debugger-X.Y.Z-chrome.zip` attached, built by the CI run that just passed. Download it from the Releases page, unzip it, and **Load unpacked** on the unzipped folder. Or:

```sh
gh release download --pattern '*-chrome.zip' --dir rsc-debugger   # the latest release
```

then unzip it and load the folder.

- The version is the next patch after the newest `vX.Y.Z` tag. It is stamped into the built manifest only; `package.json` is not changed.
- To bump the minor or major version, change `version` in `package.json` in your PR. When it is ahead of the tags it is used as it is, and patches continue from there.
- The first release is `v0.1.0`.

**CI artifacts.** Every CI run, including pull requests, uploads the production build as the `rsc-debugger-chrome-mv3` artifact.

- GitHub: open the run, download the artifact, unzip it, and **Load unpacked** on the unzipped folder.
- Or with the CLI:

  ```sh
  gh run list --workflow ci.yml --limit 5
  gh run download <run-id> -n rsc-debugger-chrome-mv3 -D rsc-debugger
  ```

  then load the `rsc-debugger` folder.

Artifacts expire after GitHub's retention period, 90 days by default.

## 4. Smoke test on a real Next.js app

The parser and the capture code are tested against recorded payloads and fakes, but **this checklist is the only thing that tests them in a real browser**. Run it with the production build.

Use any App Router app (`next build && next start` gives the simplest payload; `next dev` adds debug rows).

1. Load the extension, open the app, open DevTools, and **reload the page** with DevTools open.
2. In the page's Console (the default `top` context), run:

   ```js
   Object.getOwnPropertyDescriptor(window, '__next_f')
   ```

   It should show `get` and `set`. That means the hook was installed before Next's bootstrap. A plain `value: [...]` means the hook ran too late or not at all.
3. Open the **RSC** tab. Expect one stream named after the page URL, in state `ended`, with a list of rows (ids in hex, tags such as `model`, `module`, `text`). A real Next app should show **no red diagnostics**.
4. Click a row, expand the tree, and use **copy path** on a value. Type a key you know is in the page's props into the search box.
5. Click a client-side `<Link>` to another page. A second stream named `GET <url>?_rsc=…` should appear. This works because DevTools was open before the click.
6. Do a full navigation in the same tab (type a new URL, or click a plain `<a>`). The panel should pick up the new document. If it stays empty or keeps the old one, that is the known navigation risk below.
7. Press **Export chunks**. You get a `chunks.json` of everything captured, which is the right thing to attach to a bug report and can become a new fixture.

### What is not yet confirmed

These were reasoned from the Chrome and Next sources, not observed. A failure here is a real finding, not user error:

| Symptom | Likely cause |
|---|---|
| Step 2 shows `value`, not `get`/`set` | The MAIN-world content script did not run before Next's inline bootstrap |
| **RSC** tab is blank, no streams, and step 2 is fine | The `CustomEvent` did not cross from the MAIN to the isolated world, or `chrome.tabs.connect` from the DevTools page does not reach the relay |
| Streams appear on first load but not after a full navigation | The panel reconnects on `devtools.network.onNavigated`, which may fire before the new page's relay is listening |
| The tab never appears, or the panel is empty with a CSP error | The WASM was blocked; check the manifest's `content_security_policy` and the panel's console |

## 5. Debugging the extension itself

- **Panel:** undock DevTools (its `⋮` menu, *Dock side*), then press `Cmd+Option+I` (macOS) or `Ctrl+Shift+I` in the DevTools window. That opens DevTools for the panel page, where its console, network and sources are.
- **Hook (MAIN world):** logs appear in the inspected page's Console.
- **Relay (isolated world):** in the page's Console, pick the extension from the context dropdown above the prompt.
- **`chrome://extensions`:** the extension's card shows load and runtime errors.
- **Rust parser:** run `cargo test --manifest-path flight/Cargo.toml`. The split-at-any-byte tests and the recorded Next 16.4.3 payloads in `fixtures/` are the first place to reproduce a parse bug; save `Export chunks` output into `fixtures/` and add a test beside the existing ones that reads it.
