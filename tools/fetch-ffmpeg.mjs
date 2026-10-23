// Downloads the LGPL FFmpeg build that Soundstage bundles and checks it, so a fresh clone can build the installer.
// Usage: node tools/fetch-ffmpeg.mjs        (run from the repo root; needs Node 22+ and the `unzip`-free zip reader below)
// The build is not stored in git (it is about 134 MB). The pinned hash is of the ffmpeg.exe inside the zip.
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const URL = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-n8.1-latest-win64-lgpl-8.1.zip";
const INNER = "ffmpeg-n8.1-latest-win64-lgpl-8.1/";
const OUT = "app/src-tauri/resources/ffmpeg.exe";
// "latest" is a rolling tag, so the hash below is of the build that shipped. A newer build will differ: review it, then update this line.
const SHA256 = "dd9e8226239f522ea96352377f28a5f32ebb5dfef2f8eebe9a02e70742ec52b9";

if (existsSync(OUT) && createHash("sha256").update(readFileSync(OUT)).digest("hex") === SHA256) { console.log("ffmpeg.exe is already in place and matches."); process.exit(0); }
const dir = mkdtempSync(join(tmpdir(), "ss-ffmpeg-"));
const zip = join(dir, "ff.zip");
const res = await fetch(URL);
if (!res.ok) throw new Error(`Download failed: HTTP ${res.status}`);
writeFileSync(zip, Buffer.from(await res.arrayBuffer()));
const py = spawnSync("python", ["-I", "-c", `import zipfile,shutil,sys\nz=zipfile.ZipFile(sys.argv[1])\nwith z.open(sys.argv[2]) as s, open(sys.argv[3],'wb') as o: shutil.copyfileobj(s,o)`, zip, INNER + "bin/ffmpeg.exe", join(dir, "ffmpeg.exe")], { stdio: "inherit" });
if (py.status !== 0) throw new Error("Could not extract ffmpeg.exe (Python 3 is needed).");
const got = createHash("sha256").update(readFileSync(join(dir, "ffmpeg.exe"))).digest("hex");
if (got !== SHA256) { console.error(`The downloaded ffmpeg.exe has hash ${got}, not the pinned one. Review the new build, then update SHA256 in this file.`); process.exit(1); }
mkdirSync("app/src-tauri/resources", { recursive: true });
writeFileSync(OUT, readFileSync(join(dir, "ffmpeg.exe")));
rmSync(dir, { recursive: true, force: true });
console.log("ffmpeg.exe is in place.");
