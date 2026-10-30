# rapidraw-web changelog

Versions of the web layer ([`rrweb/VERSION`](VERSION)). Every release also names the unmodified RapidRAW inside it: `rapidraw-v<RapidRAW>-web-v<web>`. What's new in the editor itself is in [RapidRAW's releases](https://github.com/CyberTimon/RapidRAW/releases).

## 1.14.1

- **Fixed: RapidRAW Cloud's sign-in failed with "Maximum call stack size exceeded"** (in the browser console) instead of a plain "unavailable". The sign-in replaces the page's `fetch` and sends its requests through the HTTP plugin's `fetch`, which in the browser called the replaced one again, endlessly. It now uses the browser's own. Cloud sign-in still can't work from a browser (see the limitations in the README); the self-hosted AI connector does.
- Docs: the AI settings are under *Settings → General → Generative AI*.

## 1.14.0

- **RapidRAW 1.6.5** (Kelvin white balance, guided filter, pick and reject flags, reorderable panels, RapidRAW Cloud and more: see [its release](https://github.com/CyberTimon/RapidRAW/releases/tag/v1.6.5)). The automatic build for 1.6.5 had failed, because 1.6.5's cloud sign-in uses two Tauri functions the browser didn't have yet (`emit`, and `fetch` from the HTTP plugin). The build check now also looks inside the Tauri plugin packages RapidRAW uses.
- **Fixed: thumbnails could stay "loading" and photos didn't open when the connection was busy**, for example while the browser was copying a folder to the server over a slow uplink. The server dropped connections that were slow to answer its pings (after 15–30 s), and every event that arrived meanwhile, such as finished thumbnails, was lost. The server now waits 90 s, the browser 25 s. After a reconnect, the server replays the events the tab missed in the last 2 minutes.
- **Thumbnails of unedited RAWs are "light"**: RapidRAW 1.6.5 makes them from the embedded preview (now also for Fuji RAF), so they are made freely between edits. Full renders, of edited photos and other formats, still wait for the pauses and the photos next to the open one. rapidraw-web no longer writes RAF thumbnails into RapidRAW's cache itself, because RapidRAW now does the same on its own.
- Code comments are in English now, the same as in the version proposed to RapidRAW.

## 1.13.1

- **Fixed: on a Windows server, an on-demand folder with more than about 500 files never finished listing** (fewer with long file names). RapidRAW, and anything else that opened the folder, hung on it, and the whole folder stayed stuck until RapidRAW Web was restarted. Windows fetches a big listing in parts, each continuing after the last name of the previous one. The helper (`rrweb-fuse`) stored every name with an invisible character at the end, so "after the last name" always returned the last name again, and WinFsp kept asking forever. Folders of any size now list at once: 600 files in 16 ms, 1330 in 28 ms, 2000 in 37 ms on the server, plus the browser's own listing. Linux servers were not affected.

## 1.13.0

- **The filmstrip fills up while you edit.** Before, no thumbnails were made at all while a photo was open in the editor. Now they are made in short pauses between your edits (no change and no photo opening for about 1.5 s):
  - quick thumbnails (from the cache or the embedded JPEG) for every photo;
  - full renders only for the photos next to the open one in the filmstrip: the next 20 first, then the previous 5. When you move on to the next photo, its thumbnail is usually ready. For an edited photo in a folder used from another computer, rendering its thumbnail also brings the photo to the server, so it then opens faster.
  
  The other photos continue in the library. The top bar shows *Thumbnails 30 · between edits, 12 in library*.
- **It adapts to the server.** It starts with one thumbnail at a time. rapidraw-web measures how long slider previews and opening a photo take while a thumbnail is being made, compared with the time without one. If editing gets noticeably slower, it makes fewer at once and waits longer before starting (up to 8 s). If editing stays fast, it makes more at once (up to a quarter of the CPU threads, at most 4) and starts sooner. The server log (`relay.log`) records every change.

## 1.12.1

- **Fixed: everything could stop for minutes (thumbnail labels, opening a photo kept loading) when the connection between the browser and the server died silently.** In the reported case one connection stopped carrying any data at all, while the other connections between the two computers kept working. Neither side noticed for about 3 minutes, and the photo being opened at that moment failed. Now the browser checks the server whenever it hears nothing for 5 s. If there is no answer, it opens a new connection after about 19 s in total. Requests that were waiting and are safe to repeat (opening a photo, previews, listings, reading data) are sent again on the new connection instead of failing. Meanwhile the badge shows *Reconnecting…*. The server also closes connections that stop answering its pings (within 30 s).
- When several tabs or computers use the same server, a photo counts as open while it is open in any of them. Before, a tab showing the library could restart heavy thumbnails while another tab was editing.

## 1.12.0

- **Fixed: sliders sometimes took seconds to update in folders used from another computer (on demand), even on a LAN.** After every change RapidRAW saves your edit next to the photo and reads the photo's details. It does this on its main thread, so all the rendering waits until it finishes. On an on-demand folder, each such check made the server ask your browser for the whole folder listing again: about 1 s for a folder with 1330 files, and up to 10 s while the browser was also sending other data. The rendering itself took 0.1–0.5 s. The server now answers these checks from the listing it already has and refreshes it in the background. Its own changes (saved edits, renames, deletions) go straight into that listing. Checking a photo went from 1–1.2 s to 1 ms on the server. The browser also lists big folders faster.
- **Every change now shows its progress next to the slider:** a small ring with a percentage while the server renders the change and sends the preview. RapidRAW renders the whole image on the GPU in one pass and does not report how far along it is. The ring therefore fills by measured time: how long the same slider took before on this server, then the preview's transfer once the server reports it is done. If a change takes much longer than usual, the ring turns orange and shows the elapsed seconds. RapidRAW always renders only the newest state, so changes made in a row finish together with the last preview. Changes that don't come from a slider (curves, masks, crop…) show their name and progress at the top.
- Labels on thumbnails that aren't ready (*Queued #3 · ~5 s*…) moved to the top-left corner, so they no longer cover the file name.
- The server log (`relay.log`) also records RapidRAW commands that take longer than 1 s.

## 1.11.0

- **While a photo is open in the editor, no heavy thumbnails are made at all** (edited photos, which RapidRAW renders from the whole RAW on the GPU and, for folders used from another computer, after fetching the whole file). Only thumbnails RapidRAW already has in its cache still appear. Everything continues as soon as you go back to the library. Before, one at a time started after 15 s without changes and slowed the next slider move. Test with 40 edited RAFs: 9 thumbnails rendered during 30 s of editing before, 0 now.
- **Visible first:** thumbnails of the tiles you see right now (library and filmstrip) jump to the front of the queue.
- **How many are left, at the top:** *Thumbnails 37 · ~2 min* or *Thumbnails 18 · paused while editing*, also for thumbnails that aren't on screen.
- The filmstrip in the editor also shows short labels on thumbnails that aren't ready (*Paused*, *#3 · 5 s*, *Rendering*).

## 1.10.0

- **See what's happening while a photo opens.** A small panel at the top shows the photo's name and progress: for folders used from the computer you browse from, *downloading 12 MB of 29 MB · 14 MB/s · ~1 s left*; then *decoding RAW… 0.6 s (usually ~1.5 s)*.
- **Thumbnails that aren't ready yet say why:** *Queued #3 · ~5 s*, *Rendering…*, *Downloading 45 %*, *Fetching preview…* or *Paused while you edit*. The labels disappear as thumbnails arrive.
- Going back from the editor to the library resumes thumbnails at once (before, they waited 15 s after the last edit).

## 1.9.1

- Fixed: the folder pickers (*Add Folder*) and the Files tab could hang and show nothing while a folder from another computer was shared on demand but not connected (for example from a closed browser tab). Such folders are now shown as *not connected* and are not touched, operations on them fail at once instead of waiting, and RapidRAW's own checks of them no longer block for two minutes.
- The Files tab also lists folders shared from other browsers or tabs, with *Stop* (also on plain-HTTP addresses, where adding folders isn't possible).
- The exact version (`rapidraw-v<RapidRAW>-web-v<web>`, as on GitHub) is shown at the top, linked to its release.

## 1.9.0

- **Editing comes first in every folder**, not only in on-demand ones: thumbnails are handed to RapidRAW by rapidraw-web, none while you edit (and for 15 s after your last change), one at a time while a photo is open, four at a time in the library. Thumbnails of edited photos are rendered from the whole RAW on the GPU and used to slow the sliders down. Test with 60 edited RAFs in a local folder: opening a photo 5.6 → 2.9 s, slider previews 30–50 % faster, no thumbnail work while editing.
- **Fuji RAF EXIF in every folder** comes from the embedded JPEG's header (~65 KB) instead of the whole file, unless the `.rrdata` already holds EXIF.
- **Choose the preview size and live quality yourself** in the connection panel (badge), besides the recommendation.
- The badge shows **download and upload** (↓ ↑).
- Fixed: RapidRAW could crash (*bridge disconnected*) when it read a photo from a folder whose computer had been disconnected for longer than 2 minutes. Such photos now give a clear message, and short read errors are retried.
- The server part keeps a log, `logs/relay.log` in RapidRAW Web's data folder (`RR_LOG`), so problems can be traced afterwards.

## 1.8.0

- **The photo you are editing comes first.** In on-demand folders, opening a photo fetches it whole, in parallel, ahead of thumbnails and the background copy. While you edit (and for a few seconds after each change), thumbnails and the background copy wait; thumbnails of edited photos (RapidRAW renders them from the whole file on the GPU) are made at most two at a time. On a LAN test with 24 RAFs: opening a photo 3.1 → 1.9 s, first slider preview 870 → 230 ms; over a slower connection the difference is much bigger.
- **Connection badge:** measures with several parallel connections without counting the ping, so fast connections are no longer underestimated, and also measures upload (folders from the computer you browse from travel at its upload speed). Background transfers pause during the test.
- Fixed: on Linux, the server part could keep running after the RapidRAW Web window/service had stopped; leftover mounts are cleaned up only once the server owns its port (a second instance no longer detaches the mounts of the first).
- Docs: through a tunnel such as Cloudflare, traffic also passes the server's internet upload; at home the LAN address is faster (and Chrome can treat it as secure for folders from this computer).

## 1.7.0

- **On-demand folders on Windows servers.** *Use a folder from this computer → On demand* now also works when RapidRAW Web runs on Windows: install the free [WinFsp](https://winfsp.dev) once. The folder is mounted as a directory (no drive letter needed) and behaves as on Linux: only what RapidRAW reads travels, edits and exports go straight back, deleted originals go to `.rrweb-trash`, Fuji RAF browsing uses the embedded JPEG. Without WinFsp the dialog says what is missing; copy mode works as before.
- `rrweb-fuse` is included in the Windows installer and in the server bundle (`fuse/x86_64/rrweb-fuse.exe`).
- `RR_VERBOSE=1` also logs the file operations of on-demand folders.

## 1.6.0

- **Fast Fuji RAF browsing in on-demand folders.** Opening a folder of RAF files no longer pulls every whole file (30–45 MB each) from the computer you browse from. The browser sends a finished thumbnail made from the camera's embedded JPEG (about 0.3 MB) and the EXIF header (about 65 KB); RapidRAW uses them as it does for NEF/ARW. In a test with 12 RAFs: 0 MB of RAW data instead of 378 MB until a photo is opened in the editor. Only for photos without edits; edited ones are rendered by RapidRAW as before.
- The build checks that RapidRAW still caches thumbnails and reads EXIF the same way, so a RapidRAW update can't silently break this.

## 1.5.1

- Servers without a screen: the bridge no longer tries to open a browser when it runs under `xvfb-run` (or with `RR_NO_BROWSER=1`); on KDE that started `kde-open`, which crashed.

## 1.5.0

- **Linux: one file.** The `.deb`, `.rpm` and `.AppImage` are now all-in-one like on Windows and macOS: RapidRAW, the web UI, Node.js and the on-demand folder helper in one package. On a desktop, start *RapidRAW Web Bridge* from the menu; it starts the server, opens the browser and asks for your photo library.
- **Linux servers without a screen:** the `.deb`/`.rpm` install a systemd user service: `systemctl --user enable --now rapidraw-web`. The photo folder is `~/Pictures`, or set `RR_PHOTOS` with `systemctl --user edit rapidraw-web`.
- The server bundle (`…_linux_server.tar.gz` with `run.sh`) is now optional, for Docker or starting the parts yourself.
- The bundled Node.js is called `rrweb-node` (on Linux it is installed in `/usr/bin` and must not clash with a system Node.js). On Windows the firewall still asks about *Node.js JavaScript Runtime*.

## 1.4.1

- **Folders from this computer, on demand: much more robust.**
  - The server no longer stops when a connection breaks in an unusual way (for example through a Cloudflare tunnel). If it ever stops anyway, `run.sh` and the Windows/macOS app start it again, and the browser registers its folders again by itself, at the same path, so RapidRAW keeps working.
  - When the connection to the browser drops, RapidRAW waits up to 2 minutes for it to come back instead of getting read errors, which could crash it.
  - Mounts left behind by a crash (*Transport endpoint is not connected*) are cleaned up automatically.
  - Data travels in pieces of at most 1 MB with a keepalive, so tunnels and proxies don't cut the connection.
  - New option **Meanwhile copy the rest to the server in the background** (on by default): while RapidRAW isn't reading, the rest of the folder is copied to the server, so browsing gets fast. It stops when less than 5 GB is left on the server's disk.
- Corrected: thumbnails of Fuji RAF and Canon CR3 files need the whole file in on-demand mode (RapidRAW can't read their embedded preview), not about 1 MB.
- Linux: run the server as your normal user, not with `sudo`; how to keep it running in the background with `systemd-run`.

## 1.4.0

- **Use a folder from this computer.** In the Files tab (*This computer*), share a folder from the computer you are browsing from. Your photos and edits stay there; RapidRAW on the server edits them. Needs Chrome or Edge, and HTTPS or localhost.
  - **Copy to the server in the background:** the folder with all subfolders is copied to the server first (resumable, already copied files are skipped). Edits (`.rrdata`) and exports written by RapidRAW come back into the folder automatically. Works with every server OS.
  - **On demand** (Linux server): nothing is copied up front. The folder appears to RapidRAW as a disk (FUSE) and only the bytes it reads travel: a thumbnail needs about 1 MB, a photo is fetched completely once you edit it. Everything RapidRAW writes goes straight back into the folder.
  - **Keep a copy on the server** in a folder you choose, or use a temporary copy that is deleted when you stop sharing.
- **Preview quality for your connection.** A badge next to *Editor | Files* measures speed and ping, recommends RapidRAW's preview size and live quality, and warns when previews get slow.
- A relay started with a non-default `RR_PORT` no longer takes over port 8780 of another program on Windows; new `RR_BRIDGE_PORT`.

## 1.3.0

- macOS builds for Apple Silicon and Intel: all-in-one app like on Windows (experimental, not notarized by Apple).
- Release files are named `<tag>_<os>_<arch>`; the release notes list downloads per OS.

## 1.2.0

- **Photo library**: chosen once in the RapidRAW Web window on the server and always listed first.
- **Folder and file pickers** for the server in the browser (*Add Folder*, export, import, LUTs, presets) instead of typing paths.
- **Copy path** in the Files tab.

## 1.1.0

- Versioned separately from RapidRAW (`rapidraw-v<RapidRAW>-web-v<web>`), so web updates ship without waiting for a RapidRAW release.
- All-in-one Windows installer (includes Node.js; starts the server and opens the browser).
- **Files tab**: download (several files as .zip), upload, new folder, rename, copy, move, delete to the trash; RapidRAW edits travel with their photos.

## 1.0.0 (`v1.6.4-web`)

- First release: the RapidRAW interface in the browser, all processing on the server's GPU. Linux packages and server bundle, Windows bridge.
