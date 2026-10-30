# rapidraw-web

**Use [RapidRAW](https://github.com/CyberTimon/RapidRAW) in your browser, with all processing on a server in your network.**

> Unofficial fork. RapidRAW is created and maintained by **[Timon Käch (CyberTimon)](https://github.com/CyberTimon)**. This project only adds a thin web layer around it. For the desktop editor, go to the [original repository](https://github.com/CyberTimon/RapidRAW) or [getrapidraw.com](https://www.getrapidraw.com).

## The idea

I wanted one machine on my LAN that stores my photos (NAS) and has a GPU, and to edit those photos from any computer in the house, in a browser, with **exactly** the RapidRAW interface I already know: no VNC/RDP, no video stream of a remote desktop, no reimplemented "web version" with half the features.

RapidRAW makes this possible because it is a [Tauri](https://tauri.app) app: its interface is a React web app that talks to a Rust backend over IPC. rapidraw-web moves that IPC onto the network:

```
Browser (original RapidRAW UI + shim)
    │  WebSocket /ipc   JSON calls ↑   rendered previews (JPEG bytes) / events ↓
    ▼
Relay (Node.js)  ─ serves the UI, thumbnails, routes calls
    │  WebSocket /bridge (loopback only)
    ▼
RapidRAW Web Bridge  ─ the real RapidRAW backend, unmodified, rendering on the server GPU (wgpu)
    │
    ▼
GPU + photo storage
```

- **The browser shows the real RapidRAW UI.** Same React code, same panels, same sliders. Only the `@tauri-apps/*` imports are swapped for small shims at build time, so every feature that exists in RapidRAW exists here.
- **Rendering happens in RapidRAW on the server.** Moving a slider sends the adjustment; the server's GPU renders it and sends back a compressed preview — a few hundred KB, a few milliseconds on a gigabit LAN.
- **Zero changes to RapidRAW's code.** Everything lives in new files (`rrweb/`, `.github/`). The bridge is the regular RapidRAW backend that loads a 3 KB relay page instead of its UI. Upstream updates merge cleanly.

## What you get

- **The full RapidRAW editor in a browser**, on any laptop, tablet or mini PC in your network (or anywhere, through a VPN or tunnel). Nothing to install on that device.
- **All processing on the server**: RAW decoding, every slider, masks, AI tools and exports run on the server's CPU and GPU. The browser only shows the previews.
- **One-file installers**: Windows (`setup.exe`), Linux (`.deb`, `.rpm`, `.AppImage`, with a service for headless servers) and macOS (experimental). Each contains RapidRAW, the web UI and Node.js.
- **Photo library and folder pickers** for the server's folders, so you never type server paths.
- **[Files tab](#files-tab)**: download exports (also as `.zip`), upload, rename, copy, move and delete in the photo folders. RapidRAW's edit files go along.
- **[Folders from the computer you browse from](#use-a-folder-from-this-computer)**: edit photos that stay on your laptop. They are either copied to the server in the background, or mounted on demand so you can start right away (Linux and Windows servers). Edits and exports are saved back into that folder.
- **[Editing comes first](#editing-comes-first)**: thumbnails are scheduled around your work, the visible ones first, and while a photo is open only in the pauses between edits. Thumbnails of the photos next to it in the filmstrip come first. How much runs at once adapts to the server.
- **Fast Fuji RAF browsing**: the library's EXIF comes from the embedded JPEG's 65 KB header instead of the whole 30–45 MB file (thumbnails of unedited RAWs come from the embedded preview in RapidRAW itself since 1.6.5).
- **You can see what is happening**: progress while a photo opens (download, then decoding), labels on thumbnails that aren't ready, the number of thumbnails left in the top bar, and a ring with a percentage next to the slider you just moved.
- **[Connection badge](#connection-and-preview-quality)**: measures download, upload and ping, and recommends RapidRAW's preview size and quality for your connection. If a connection dies silently, it shows *Reconnecting…*, and the browser reconnects and repeats what was waiting.
- **The exact version** (`rapidraw-v<RapidRAW>-web-v<web>`) is shown at the top, linked to its release.
- **Always the current RapidRAW**: every RapidRAW release is rebuilt automatically with this web layer.

## How it differs from RapidRAW on your computer

| | RapidRAW (desktop app) | rapidraw-web |
|---|---|---|
| Where the editor runs | A window on the computer in front of you | A browser tab on any device; the editor is the same React UI |
| Who does the work | That computer's CPU and GPU | The server's CPU and GPU |
| Where the photos are | That computer's disks | The server's disks, or a folder on the browsing computer (copied or mounted on demand) |
| Thumbnails | Several workers start as soon as a folder opens, even while you edit | Scheduled around your editing, the visible ones and the photos next to the open one first, adapted to the server |
| Fuji RAF EXIF | Read through rawler from the whole file | From the embedded JPEG's 65 KB header |
| Slider preview | Drawn directly in the window (on Windows and macOS straight from the GPU) | Rendered on the server, sent as a JPEG (a few ms on a LAN) |
| Extras | — | Files tab, folders from the browsing computer, progress labels, connection badge |

## Why it can feel faster than RapidRAW installed on your laptop

It is the same RapidRAW, and its image processing takes about the same time in both. What changes is what else runs next to it, and where:

1. **Your photo gets the machine to itself.** Desktop RapidRAW makes thumbnails with several workers as soon as a folder opens. For an edited photo, each thumbnail is a whole-RAW decode plus a GPU render (in RapidRAW 1.6.4 also for every Fuji RAF), competing with the photo you are editing. In one desktop session on a test server (i7-9750H, GTX 1660 Ti), the first preview of the opened photo took 39.5 s while six thumbnails were being decoded next to it. rapidraw-web hands thumbnails over in small numbers and makes them only in the pauses while you edit. In a test with 60 edited Fuji RAFs, opening a photo went from 5.6 to 2.9 s, and slider previews got 30–50 % faster than with RapidRAW's own scheduling on the same machine.
2. **The interface runs in your browser, not next to the processing.** On Linux, desktop RapidRAW draws its interface with WebKitGTK, the web engine Tauri uses there. It shares the CPU and GPU with the RAW processing and is usually slower than Chrome or Edge at drawing a heavy interface, especially with NVIDIA drivers, where it often can't draw with the GPU. With rapidraw-web the interface runs in your browser, on the device in front of you, and the server only processes. Even with the browser on the server itself, the interface then runs in Chrome or Edge instead of WebKitGTK.
3. **Stronger hardware can do the work.** Every slider change is a full GPU render, and opening a RAW is CPU-heavy decoding. If the device in front of you is a thin laptop with integrated graphics, a server with a dedicated GPU can do both several times faster, and the laptop only has to show a JPEG.
4. **Less work for Fuji RAF libraries.** The library's EXIF data comes from a 65 KB header instead of the whole 30–45 MB file.
5. **Lighter previews where they help.** The connection badge recommends RapidRAW's preview size and live preview quality for your connection, or you choose them yourself. A lighter live quality (*Performance*) makes the previews while you drag a slider smaller and faster to render, on any machine. Desktop RapidRAW uses its defaults (*High*, 1920 px) unless you change them in *Settings → Processing*, so compare with the same settings.

Where it is *not* faster: each preview has to travel over the network, which takes a few milliseconds on a LAN and more over the internet. On the same powerful computer, desktop RapidRAW shows a change without that trip. rapidraw-web helps most when the device in front of you is weaker than the server, or when your photos live on the server.

## Quick start

Download from the [Releases](https://github.com/vedranius/rapidraw-web/releases) page. Any GPU with Vulkan, DX12 or Metal works, including integrated Intel graphics.

### Windows x64: all-in-one installer

1. Run `rapidraw-v*-web-v*_windows_x64-setup.exe`. It contains everything (RapidRAW, the web UI and Node.js), nothing else to install.
2. Start **RapidRAW Web Bridge** from the Start menu. It starts the server and opens RapidRAW in your browser.
3. In the small RapidRAW Web window, choose your **photo library**: the folder on this computer that holds your photo folders. The window also lists the addresses for other devices on your network (`http://<this-pc>:8780`). Closing it stops the server.

The first time, Windows Firewall asks about *Node.js JavaScript Runtime*: allow **Private networks** to reach it from other devices.

To use folders from other computers **on demand** (see [below](#use-a-folder-from-this-computer)), also install the free [WinFsp](https://winfsp.dev) once (`winfsp-*.msi` from its [releases](https://github.com/winfsp/winfsp/releases/latest), default options). Copy mode works without it.

### macOS: all-in-one app (experimental)

1. Open `…_macos_arm64.dmg` (Apple Silicon) or `…_macos_x64.dmg` (Intel) and drag **RapidRAW Web Bridge** to *Applications*.
2. The app is not notarized by Apple: the first time, right-click it → *Open*, or allow it under *System Settings → Privacy & Security → Open Anyway*.
3. Same as on Windows from here: it opens RapidRAW in your browser and asks for your photo library. Allow *rrweb-node* to accept incoming network connections to use it from other devices.

### Linux: one package

Download **one** file for your distribution (arm64: `…_linux_arm64.*`). Each contains everything: RapidRAW, the web UI, Node.js and the helper for on-demand folders.

```bash
sudo apt install ./*_linux_x64.deb      # Debian / Ubuntu
sudo dnf install ./*_linux_x64.rpm      # Fedora / openSUSE (zypper)
chmod +x ./*_linux_x64.AppImage         # any distro, nothing to install: just run it
```

**On a desktop:** start **RapidRAW Web Bridge** from the menu (or run the AppImage). Same as on Windows: it starts the server, opens RapidRAW in your browser and asks for your photo library.

**On a server without a screen** (`.deb`/`.rpm`; uses `xvfb`, installed as a recommended package):

```bash
systemctl --user enable --now rapidraw-web    # starts now and at every login
sudo loginctl enable-linger $USER             # …and at boot, without logging in
journalctl --user -u rapidraw-web -f          # logs
```

The photo library is `~/Pictures`. To use another folder (or set any of the [variables below](#configuration)), run `systemctl --user edit rapidraw-web`, add the lines below and restart it with `systemctl --user restart rapidraw-web`:

```ini
[Service]
Environment=RR_PHOTOS=/mnt/photos
```

With the AppImage on a server: `RR_PHOTOS=/mnt/photos xvfb-run -a ./…_linux_x64.AppImage`. The GPU is used through Vulkan and doesn't need a display; `xvfb` only gives the bridge's small window somewhere to live.

Open `http://<server>:8780`. Run it as your normal user, not as root: as root, edits and exports in your photo folders become root's files and RapidRAW's settings end up in `/root`. If the server part ever stops unexpectedly, the bridge starts it again, and the browser reconnects by itself.

Updating: install the newer package the same way. Its version is `<RapidRAW>+web.<web>` (e.g. `1.6.4+web.1.5.0`), so a new web release is an upgrade even when RapidRAW itself is unchanged.

**Server bundle** (`…_linux_server.tar.gz`, optional): the relay and web UI on their own, started with `run.sh` (needs Node.js 20+ and a bridge from one of the packages above, which `run.sh` finds by itself). Useful for Docker or if you want to start the parts yourself: `RR_PHOTOS=/mnt/photos ./run.sh`. Without a display it starts the bridge under `xvfb-run`, and it restarts the relay if it stops. It also works on Windows (`powershell -ExecutionPolicy Bypass -File .un.ps1`).

## Photo library and folder pickers

The **photo library** is the top folder of your photos on the server. Choose it in the RapidRAW Web window on the server (*Choose…*, a normal folder dialog of that computer); with `run.sh`/`run.ps1`, `RR_PHOTOS` does the same. You can change it any time.

In the browser, every place where RapidRAW asks for a folder or file (*Add Folder*, export destination, import, LUTs, presets) opens a picker for the server that starts in the photo library, so you never need to know a server path. *Type a path…* in the picker still accepts any path.

## Files tab

Next to the editor there is a **Files** tab (top centre): a file manager for the photo folders on the server, so you can get your exports out and organise shoots without another tool.

- Browse the photo library and the folders you opened in RapidRAW, with breadcrumbs, sorting and multi-select (Ctrl/Shift-click, Ctrl+A).
- **Download** a file, or several files and folders as one `.zip`; **Upload** files with the button or drag & drop.
- **New folder**, **Rename** (F2), **Copy/Cut → Paste** (Ctrl+C/X/V) between folders, **Delete** to the server's trash.
- **Copy path** copies the server path of the selected item or of the current folder.
- A photo's RapidRAW edits (`.rrdata`, virtual copies, `.rrexif`) move, copy, rename and delete together with it. *Show edit files* reveals them.
- When you switch back to **Editor**, RapidRAW reloads the current folder.

The Files tab only reaches inside those photo folders, and deleting goes to the trash, so it can be recovered.

## Use a folder from this computer

Your photos don't have to be on the server. In the Files tab, under **This computer**, choose a folder on the computer you are browsing from, for example a laptop in the field. The photos and every edit stay in that folder; RapidRAW on the server does the processing. In the editor, open it with *Add Folder* → *‹folder› (this computer)*.

Pick one of two modes:

| | **Copy to the server in the background** | **On demand** |
|---|---|---|
| What travels | The whole folder with subfolders, copied first (resumable) | Only what RapidRAW reads: for browsing, the embedded preview of each RAW (a few MB at most); the whole photo once you edit it |
| When you can edit | As photos arrive | Right away |
| Best for | Time to wait, or a slow upload you leave running | Editing a few photos quickly on a fast connection |
| Server | Any OS | Linux with `fuse3`, or Windows with [WinFsp](https://winfsp.dev) (macOS: see below) |

In on-demand mode, **Meanwhile copy the rest to the server in the background** (on by default) fetches the rest of the folder whenever RapidRAW isn't reading anything, so browsing gets as fast as with photos on the server. It pauses while you work and stops when less than 5 GB is left on the server's disk.

In both modes, everything RapidRAW writes (edits in `.rrdata`, exports, new folders) is saved back into the folder on this computer automatically. With **Keep a copy on the server**, the photos that reached the server and all edits also stay in a server folder you choose; otherwise the server copy is temporary and deleted when you click *Stop*.

**Requirements**
- **Chrome or Edge**: only they let a web page read and write a folder you pick (File System Access API). Firefox and Safari can't.
- **HTTPS, or `localhost`**: browsers allow folder access only on secure pages. The easiest way is [Tailscale](https://tailscale.com): on the server run `tailscale serve --bg 8780` and open `https://<server>.<tailnet>.ts.net`. That works from anywhere and keeps the server private. A reverse proxy with a certificate (Caddy, nginx) works too. On your own LAN you can also tell Chrome to treat the plain address as secure: open `chrome://flags/#unsafely-treat-insecure-origin-as-secure`, add `http://<server>:8780` and restart Chrome.
- **Keep the tab open** while editing: the browser is the storage. If it closes, a copy in progress pauses and continues when you reconnect. If the connection drops (Wi-Fi, a tunnel, reloading the page), RapidRAW waits up to 2 minutes for it to come back instead of getting read errors, and after a server restart the browser registers its folders again by itself.

**How it works.** The browser acts as the storage: it reads and writes the chosen folder for the server over a WebSocket. *Copy* mode uploads the files into a mirror folder on the server, and a watcher sends new or changed files back. *On demand* mode mounts the folder as a disk on the server (`rrweb-fuse`, included in every Linux and Windows package: FUSE on Linux, WinFsp on Windows); each read fetches just the needed range from the browser into a server-side cache, with read-ahead while RapidRAW decodes a photo. Data travels in pieces of at most 1 MB with a keepalive, so tunnels and proxies such as Cloudflare don't cut the connection; the photo you open in the editor comes first (it is fetched whole, in parallel, ahead of everything else), then thumbnails, then other reads, then the background copy. While you edit (and for 15 seconds after each change) thumbnails and the background copy wait, so sliders get the whole connection, CPU and GPU (see [Editing comes first](#editing-comes-first)). The server keeps each folder's listing and refreshes it in the background, so RapidRAW's checks of single files (it makes one after every edit, on its main thread) don't wait for the browser. Listing a big folder takes about a second. If the computer that shares the folder disconnects for longer, its photos can't be opened until it reconnects (RapidRAW says so instead of waiting). Mounts left behind by a crash are cleaned up when the server starts.

**Fuji RAF.** The library reads the EXIF of every photo in a folder as soon as you open it, and for a RAF RapidRAW reads far more of the file than the metadata: in on-demand mode that would mean most of 30–45 MB per photo. So for RAF files, the browser sends only the EXIF header of the camera's embedded JPEG (about 65 KB) and RapidRAW reads the EXIF from that. Thumbnails of unedited RAWs come from the embedded preview, which RapidRAW reads itself (since 1.6.5 also for RAF).

**Safety.** The page can only reach the folder you picked (the browser asks for permission). Deletions are never mirrored as permanent deletes of your originals: in copy mode they are not sent back at all, and in on-demand mode a deleted original is moved to a hidden `.rrweb-trash` folder inside the shared folder.

**On-demand mode on other server OSes.** Copy mode already works on every server. On-demand mode needs a user-space file system on the server:

| Server | Status | What it needs |
|---|---|---|
| Linux | ✅ Supported | The `fuse3` package (`fusermount3`), usually already installed |
| Windows | ✅ Supported | [WinFsp](https://winfsp.dev) (free file system driver, install once). The folder appears as a directory under RapidRAW Web's cache folder; names are case-sensitive there, like on the computer that shares it |
| macOS | Planned | [macFUSE](https://osxfuse.github.io) (kernel extension, must be allowed in macOS security settings, on Apple Silicon also in Recovery) or [FUSE-T](https://www.fuse-t.org) (no kernel extension) |

## Editing comes first

RapidRAW makes thumbnails in the background with several workers; for an edited photo it renders the whole RAW on the GPU, the same GPU that renders your slider changes. rapidraw-web therefore hands thumbnails to RapidRAW itself. In the library it hands over four at a time, the ones you can see first. While a photo is open in the editor, it works only in short pauses between your edits (no change and no photo opening for about 1.5 s): quick thumbnails (cached or from the embedded JPEG) for every photo, and full renders only for the photos next to the open one in the filmstrip (the next 20, then the previous 5), so the filmstrip fills up as you go from photo to photo. How many it makes at once and how long it waits adapt to the server: it measures slider previews and photo opening while a thumbnail is being made. If they get slower, it makes fewer and waits longer; if they stay fast, it makes more (up to a quarter of the CPU threads, at most 4). The top bar shows how many are left (*Thumbnails 37 · ~2 min*, or *Thumbnails 30 · between edits, 12 in library*). In a test with 60 edited Fuji RAFs in a local folder, opening a photo went from 5.6 to 2.9 seconds and slider previews got 30–50 % faster. While a photo opens, a panel at the top shows its progress (download from the computer you browse from, then decoding), and thumbnails that aren't ready yet say where they are (*Queued #3 · ~5 s*, *Rendering…*, *Paused while you edit*). The library's EXIF data of Fuji RAF files is read from the embedded JPEG's header (about 65 KB) instead of the whole file, unless the photo's `.rrdata` already holds it.

**Feedback while you edit.** Next to the slider you changed, a small ring with a percentage shows the change being rendered on the server and the preview on its way. RapidRAW renders the whole image on the GPU in one pass and does not report how far along it is, so the ring fills by measured time: how long the same slider took before on this server, then the transfer of the preview (the server reports when it is done and how big the preview is). If a change takes much longer than usual, the ring turns orange and shows the elapsed seconds. RapidRAW always renders only the newest state and skips the ones in between, so several changes in a row finish together with the last preview. Changes that don't come from a slider (curves, masks, crop…) show their name and progress at the top. The server log records RapidRAW commands that take longer than a second.

## Connection and preview quality

The badge next to *Editor | Files* shows the measured speed to the server. Once per browser tab it measures download and upload speed (several parallel connections, like a speed test) and ping, and recommends RapidRAW's **preview size** and **live preview quality** (*Settings → Processing*). Download carries the previews; folders used from the computer you browse from travel at its upload speed.

If the connection to the server stops answering (a connection can die silently, without either side noticing), the badge shows *Reconnecting…*. The browser opens a new connection within about 20 seconds and repeats the requests that were waiting, such as opening a photo or a preview.

Through a tunnel such as Cloudflare, everything between the server and your browser also passes the server's internet **upload**, even when you are at home on the same network. At home, the LAN address (`http://<server>:8780`) is usually much faster.

| Connection | Recommended |
|---|---|
| under 5 Mbit/s, or ping over 200 ms | 1280 px, Performance |
| 5–15 Mbit/s (mobile) | 1920 px, Performance |
| 15–50 Mbit/s | 1920 px, High |
| 50 Mbit/s and more (LAN) | 2560 px, High |

*Use recommended*, or pick a preview **size** and **live quality** yourself and *Save*; both reload the editor. While you edit, the badge also watches how long real previews take and turns amber when they get slow.

### Configuration

| Variable | Default | |
|---|---|---|
| `RR_PHOTOS` | `~/Pictures` in the Linux service; required by `run.sh`/`run.ps1` | Photo folder on the server, shown in the Files tab and the pickers (the library chosen in the RapidRAW Web window comes first) |
| `RR_PORT` / `RR_HOST` | `8780` / `0.0.0.0` | Listen address |
| `RR_AUTH` | off | `user:pass` → HTTP Basic auth for UI, files and IPC |
| `RR_CONFIG` | `<bridge data dir>/rrweb.json` | Where the photo library chosen in the RapidRAW Web window is stored |
| `RR_ORIGINS` | — | Extra allowed browser origins, comma-separated, e.g. `https://photos.example.com` behind a reverse proxy that changes the `Host` header |
| `RR_ROOTS` | photos + bridge data/cache dirs | Directories `/files` is allowed to serve |
| `RR_WORK` | `<bridge cache dir>/remote` | Server-side mirror, cache and mount points for folders from browsing computers |
| `RR_FUSE_BIN` | next to the bundled Node.js, or `fuse/<arch>/rrweb-fuse[.exe]` in the server bundle | Helper for on-demand folders (Linux: FUSE, Windows: WinFsp) |
| `RR_BRIDGE_PORT` | `8780` | Loopback port the bridge connects to (change only together with a rebuilt bridge) |
| `RR_BRIDGE_BIN` | auto-detect | Path to `rapidraw-web-bridge` |
| `RR_VERBOSE` | off | Log every IPC call with timing, and the file operations of on-demand folders |
| `RR_LOG` | `<data dir>/logs/relay.log` (bridge, `run.sh`, `run.ps1`) | Copy of the server's log (up to 5 MB, then `.1`) |
| `RR_NO_BROWSER` | off (on in the Linux service and under `xvfb-run`) | Don't open a browser when the bridge starts its bundled server |

### GPU in Docker (NVIDIA)

Vulkan inside a container needs the `graphics` driver capability, not just `compute`:

```yaml
runtime: nvidia
environment:
  NVIDIA_VISIBLE_DEVICES: all
  NVIDIA_DRIVER_CAPABILITIES: graphics,compute,utility
```

## Security

The IPC channel can do everything RapidRAW can do on the server (browse and write files, export, delete). **Do not expose it to the internet without `RR_AUTH` and a TLS reverse proxy**, or keep it behind a VPN. The bridge port accepts loopback connections only, `/files` serves only `RR_ROOTS`, the Files tab stays inside the photo folders, and requests from other websites (a foreign `Origin`) are refused. A folder from a browsing computer is reachable only while that browser tab shares it, and only inside the folder that was picked there.

## Limitations

- One active editing session at a time: RapidRAW's backend state is global, so two browsers editing different photos at once will interfere.
- The folder/file pickers browse only the photo folders; other server paths can be typed in with *Type a path…*.
- Desktop-only window features (window controls, native drag & drop from your OS) are no-ops. Tethering is not included.
- *RapidRAW Cloud* (the optional paid generative cleanup) can't sign in from a browser: its sign-in is the desktop app's, and sends requests a browser isn't allowed to make to another site (CORS), so RapidRAW shows it as unavailable. A self-hosted *AI connector* (RapidRAW-AI-Connector with ComfyUI, *Settings → General → Generative AI → AI Connector*) works: RapidRAW on the server talks to it, so it can run on the same server.
- RapidRAW's folder tree shows folders created in the Files tab after you reopen the parent folder. Uploading whole folders (as opposed to files) is not supported yet.
- *Use a folder from this computer* needs Chrome or Edge and HTTPS (or `localhost`); on-demand mode needs a Linux or Windows server for now.

## Building from source

```bash
git clone https://github.com/vedranius/rapidraw-web.git && cd rapidraw-web
./rrweb/build.sh                                  # needs Rust, Node 20+, webkit2gtk-4.1 dev; all-in-one bridge (Linux: with rrweb-fuse)
BUNDLES=deb,rpm,appimage ./rrweb/build.sh         # also build installer packages
src-tauri/target/release/rapidraw-web-bridge      # start it (or, with the relay in a terminal: RR_PHOTOS=~/Pictures RR_VERBOSE=1 ./rrweb/run.sh)
```

## Versioning and staying in sync with RapidRAW

This repository is a GitHub fork of [CyberTimon/RapidRAW](https://github.com/CyberTimon/RapidRAW). A release carries two versions: **`rapidraw-v<RapidRAW>-web-v<rapidraw-web>`**, e.g. `rapidraw-v1.6.4-web-v1.1.0` is the unmodified RapidRAW `v1.6.4` with version `1.1.0` of this web layer. The web layer has its own version in [`rrweb/VERSION`](../rrweb/VERSION), so it can ship several updates for the same RapidRAW release.

Every release is **exactly an upstream release's code** plus one commit with the `rrweb/` layer, built and published by `rrweb-release.yml`. Releases come from two places (`rrweb-sync.yml`):
- **A new web version**: raising `rrweb/VERSION` on `main` (e.g. `1.1.0` → `1.1.1`) releases the current web layer on the latest RapidRAW release right away. The release notes list the web-layer commits since the previous web version.
- **A new RapidRAW release**: a daily job merges upstream `main` and, when RapidRAW publishes a release, releases it with the latest *published* web version (unreleased work on `main` waits for the next version bump).

What changed in each web version: [CHANGELOG](../rrweb/CHANGELOG.md).

Because rapidraw-web only adds files, upstream changes merge without conflicts. Before every build, `rrweb/check-shims.mjs` verifies that every Tauri API the RapidRAW UI imports is covered by the browser shims, and the list of forwarded backend events is regenerated from RapidRAW's source. If something new appears upstream, the build fails loudly and opens an issue instead of shipping a broken release.

## Thank you, Timon

RapidRAW is a beautiful, fast, genuinely free and open-source alternative to commercial RAW editors, built with remarkable care and pace. None of this would exist without it. If you use rapidraw-web, please ⭐ [star RapidRAW](https://github.com/CyberTimon/RapidRAW) and consider supporting its author on [Ko-fi](https://ko-fi.com/cybertimon).

## License

[AGPL-3.0](https://github.com/vedranius/rapidraw-web/blob/main/LICENSE), same as RapidRAW. If you run a modified version as a network service for others, the AGPL requires you to offer them its source code.
