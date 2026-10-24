// Downloads the offline speech program that Soundstage bundles (whisper.cpp, MIT licence) and checks it, so a fresh clone can
// build the installer. Usage: node tools/fetch-whisper.mjs   (from the repo root; needs Node 22+ and Python 3 to unzip)
// Only the command-line program and the libraries it loads are kept (about 7 MB). The speech models are NOT bundled: people
// download one inside the app, and it is checked against the size and SHA-256 listed in crates/soundstage-core/src/adapters/whisper.rs.
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const URL = "https://github.com/ggml-org/whisper.cpp/releases/download/v1.9.2/whisper-bin-x64.zip";
const SHA256 = "49dcc16de826f20bd53d44f947a1ae49dfa81f86cad67a64d80820cb192d674a";
const OUT = "app/src-tauri/resources/whisper";

if (existsSync(join(OUT, "whisper-cli.exe")) && existsSync(join(OUT, "whisper.dll"))) { console.log("The offline speech program is already in place."); process.exit(0); }
const dir = mkdtempSync(join(tmpdir(), "ss-whisper-"));
const zip = join(dir, "w.zip");
const res = await fetch(URL);
if (!res.ok) throw new Error(`Download failed: HTTP ${res.status}`);
writeFileSync(zip, Buffer.from(await res.arrayBuffer()));
const got = createHash("sha256").update(readFileSync(zip)).digest("hex");
if (got !== SHA256) { console.error(`The downloaded zip has hash ${got}, not the pinned one. Review it, then update SHA256 in this file.`); process.exit(1); }
mkdirSync(OUT, { recursive: true });
const py = spawnSync("python", ["-I", "-c", `
import zipfile, os, sys
z = zipfile.ZipFile(sys.argv[1]); out = sys.argv[2]
keep = lambda b: b in ("whisper-cli.exe", "whisper.dll", "ggml.dll", "ggml-base.dll") or b.startswith("ggml-cpu-")
for n in z.namelist():
    b = os.path.basename(n)
    if keep(b): open(os.path.join(out, b), "wb").write(z.read(n))
`, zip, OUT], { stdio: "inherit" });
if (py.status !== 0) throw new Error("Could not unzip (Python 3 is needed).");
writeFileSync(join(OUT, "WHISPER-LICENSE.txt"), `MIT License

Copyright (c) 2023-2026 The ggml authors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

whisper.cpp: https://github.com/ggml-org/whisper.cpp (release v1.9.2).
The Whisper models are by OpenAI, also under the MIT licence: https://github.com/openai/whisper
`);
rmSync(dir, { recursive: true, force: true });
console.log(`Offline speech program in place: ${readdirSync(OUT).length} files.`);
