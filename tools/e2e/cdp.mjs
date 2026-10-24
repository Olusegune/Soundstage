// A tiny Chrome DevTools Protocol driver for the built Soundstage app (WebView2 exposes CDP when started with
// --remote-debugging-port). No dependencies: Node 22+ has fetch and WebSocket built in.
import { spawn } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const KEYS = {
  Enter: ["Enter", "Enter", 13], Space: [" ", "Space", 32], Left: ["ArrowLeft", "ArrowLeft", 37], Right: ["ArrowRight", "ArrowRight", 39],
  Esc: ["Escape", "Escape", 27], F1: ["F1", "F1", 112], "/": ["/", "Slash", 191], ",": [",", "Comma", 188], Tab: ["Tab", "Tab", 9],
};

export class App {
  constructor(exe, port = 9400 + Math.floor(Math.random() * 500)) { this.exe = exe; this.port = port; this.results = []; this.id = 0; this.pending = new Map(); }

  async launch() {
    this.child = spawn(this.exe, [], { env: { ...process.env, WEBVIEW2_USER_DATA_FOLDER: process.env.E2E_PROFILE ?? `${process.env.TEMP}/soundstage-e2e-profile`, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${this.port}` }, stdio: "ignore" });
    let page;
    for (let i = 0; i < 60 && !page; i++) {
      try { page = (await (await fetch(`http://127.0.0.1:${this.port}/json`)).json()).find((t) => t.type === "page" && !/^chrome-extension:/.test(t.url)); } catch { /* not up yet */ }
      if (!page) await sleep(500);
    }
    if (!page) throw new Error("The app did not start or did not expose a debug port.");
    this.ws = new WebSocket(page.webSocketDebuggerUrl);
    await new Promise((r, j) => { this.ws.onopen = r; this.ws.onerror = j; });
    this.pageErrors = [];
    this.ws.onmessage = (m) => { const d = JSON.parse(m.data); if (d.method === "Runtime.exceptionThrown") this.pageErrors.push(String(d.params?.exceptionDetails?.exception?.description ?? d.params?.exceptionDetails?.text).slice(0, 300)); if (d.method === "Runtime.consoleAPICalled" && d.params?.type === "error") this.pageErrors.push(d.params.args.map((a) => a.description ?? a.value).join(" ").slice(0, 300)); if (d.id && this.pending.has(d.id)) { this.pending.get(d.id)(d); this.pending.delete(d.id); } };
    await this.send("Runtime.enable");
    await this.send("Page.enable");
  }

  send(method, params = {}) { return new Promise((res) => { const i = ++this.id; this.pending.set(i, res); this.ws.send(JSON.stringify({ id: i, method, params })); }); }

  async ev(expr) {
    const r = await this.send("Runtime.evaluate", { expression: expr, awaitPromise: true, returnByValue: true });
    if (!r.result) throw new Error("CDP: " + JSON.stringify(r).slice(0, 300));
    if (r.result.exceptionDetails) throw new Error(String(r.result.exceptionDetails.exception?.description ?? JSON.stringify(r.result.exceptionDetails)).slice(0, 400));
    return r.result.result.value;
  }

  async waitFor(expr, ms = 20000) {
    const t = Date.now();
    while (Date.now() - t < ms) { if (await this.ev(`!!(${expr})`).catch(() => false)) return true; await sleep(120); }
    return false;
  }

  text() { return this.ev("document.body.textContent"); }

  /** Press a real key chord, for example "Ctrl+K", "Alt+Right", "Ctrl+Shift+M", "F1". */
  async press(combo) {
    const parts = combo.split("+");
    const main = parts.pop();
    let mods = 0;
    for (const p of parts) mods |= p === "Alt" ? 1 : p === "Ctrl" ? 2 : p === "Shift" ? 8 : 0;
    let [key, code, vk] = KEYS[main] ?? (/^[A-Za-z]$/.test(main) ? [mods & 8 ? main.toUpperCase() : main.toLowerCase(), `Key${main.toUpperCase()}`, main.toUpperCase().charCodeAt(0)] : /^\d$/.test(main) ? [main, `Digit${main}`, 48 + Number(main)] : [main, main, 0]);
    const base = { modifiers: mods, key, code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk };
    await this.send("Input.dispatchKeyEvent", { type: "rawKeyDown", ...base });
    await this.send("Input.dispatchKeyEvent", { type: "keyUp", ...base });
    await sleep(60);
  }

  clickText(t, scope = "document") {
    return this.ev(`(()=>{const b=[...${scope}.querySelectorAll('button')].find(b=>b.textContent.trim().startsWith(${JSON.stringify(t)})&&!b.disabled);if(!b)return false;b.click();return true})()`);
  }
  clickSel(sel) { return this.ev(`(()=>{const e=document.querySelector(${JSON.stringify(sel)});if(!e||e.disabled)return false;e.click();return true})()`); }
  setSelect(sel, v) {
    return this.ev(`(()=>{const e=document.querySelector(${JSON.stringify(sel)});if(!e)return false;Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype,'value').set.call(e,${JSON.stringify(v)});e.dispatchEvent(new Event('change',{bubbles:true}));return true})()`);
  }
  setInput(sel, v) {
    return this.ev(`(()=>{const e=document.querySelector(${JSON.stringify(sel)});if(!e)return false;Object.getOwnPropertyDescriptor(e.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype,'value').set.call(e,${JSON.stringify(v)});e.dispatchEvent(new Event('input',{bubbles:true}));return true})()`);
  }

  async resize(w, h) { await this.send("Emulation.setDeviceMetricsOverride", { width: w, height: h, deviceScaleFactor: 1, mobile: false }); await sleep(250); }
  async resetSize() { await this.send("Emulation.clearDeviceMetricsOverride"); await sleep(200); }

  async shot(path) {
    const r = await this.send("Page.captureScreenshot", { format: "png" });
    mkdirSync(path.replace(/[\\/][^\\/]+$/, ""), { recursive: true });
    writeFileSync(path, Buffer.from(r.result.data, "base64"));
  }

  check(name, ok, extra = "") { if (!ok && this.pageErrors?.length) extra = `${extra} [page error: ${this.pageErrors[this.pageErrors.length - 1]}]`; this.results.push({ name, ok: !!ok, extra: String(extra) }); console.log(ok ? "PASS" : "FAIL", name, extra ? `| ${extra}` : ""); }

  async close() { try { this.ws?.close(); } catch { /* closed */ } this.child?.kill(); }
}
