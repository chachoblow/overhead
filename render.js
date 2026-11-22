#!/usr/bin/env node
// Headless render of the radar lab: evaluates the page's drawing code in Node and writes a 1-bit PNG.
// Usage: node render.js "<config line or preset name>" [--fov 8] [--minutes 4] [--sel NORAD] [--scale 2] [--out shot.png]
//   config line: the "Config to send back" text from the page, e.g. "marker=diamondfill7r below=pluscornershalo ..."
//   --minutes: offset from the lab's start time (2026-11-03T02:00Z); --fov: degrees (smaller = zoomed in)
const fs = require("fs"), path = require("path"), zlib = require("zlib");
const args = process.argv.slice(2), opt = k => { const i = args.indexOf("--" + k); return i >= 0 ? args[i + 1] : null; };
const html = fs.readFileSync(path.join(__dirname, "radar-lab.html"), "utf8");
const scripts = [...html.matchAll(/<script>([\s\S]*?)<\/script>/g)].map(m => m[1]);
const code = scripts.join("\n").split("// ---------- UI ----------")[0].replace('"use strict";', "") +
  "\n;globalThis.__lab = { CURRENT, PRESETS, state, computeWorld, drawRadar, drawStrip, drawInset, FB, inViewIds };";
global.performance = { now: () => 0 };
(0, eval)(code);
const L = globalThis.__lab;
const spec = args[0] || "Favourite C";
const preset = L.PRESETS.find(p => p[0] === spec);
const over = preset ? preset[1] : Object.fromEntries(spec.trim().split(/\s+/).map(kv => kv.split("=")));
const c = { ...L.CURRENT, ...over };
L.state.fov = Number(opt("fov") ?? 8);
L.state.t += Number(opt("minutes") ?? 4) * 60000;
const world = L.computeWorld(L.state.t);
L.state.sel = opt("sel") ? Number(opt("sel")) : L.inViewIds(world, c)[0];
const fb = new L.FB(); const { sel } = L.drawRadar(fb, c, world); L.drawStrip(fb, c, world, sel); L.drawInset(fb, c, world, sel);
// PNG, greyscale, memory-LCD-ish colours, nearest-neighbour scaled
const S = Number(opt("scale") ?? 2), W = 400 * S, H = 240 * S, raw = Buffer.alloc((W + 1) * H);
for (let y = 0; y < H; y++) { raw[y * (W + 1)] = 0; for (let x = 0; x < W; x++) raw[y * (W + 1) + 1 + x] = fb.b[Math.floor(y / S) * 400 + Math.floor(x / S)] ? 30 : 185; }
const crcT = new Int32Array(256).map((_, n) => { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; return c; });
const crc = b => { let c = -1; for (const v of b) c = crcT[(c ^ v) & 255] ^ (c >>> 8); return (c ^ -1) >>> 0; };
const chunk = (t, d) => { const len = Buffer.alloc(4); len.writeUInt32BE(d.length); const td = Buffer.concat([Buffer.from(t), d]); const cr = Buffer.alloc(4); cr.writeUInt32BE(crc(td)); return Buffer.concat([len, td, cr]); };
const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(W, 0); ihdr.writeUInt32BE(H, 4); ihdr[8] = 8; ihdr[9] = 0;
const out = opt("out") || "shot.png";
fs.writeFileSync(out, Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk("IHDR", ihdr), chunk("IDAT", zlib.deflateSync(raw)), chunk("IEND", Buffer.alloc(0))]));
console.log(`${out}: ${preset ? "preset " + spec : "config"}, fov ${L.state.fov}°, selected ${sel ? sel.s.id + " " + sel.s.name : "none"}`);
