// CI smoke test for the real wasm build: boots the module in Node, plays a
// little, opens every window, validates the draw buffer, and round-trips a save.
import fs from 'node:fs';

const bytes = fs.readFileSync(process.argv[2] || 'dist/brodilka.wasm');
const dec = new TextDecoder(), enc = new TextEncoder();
let ex;
const { instance } = await WebAssembly.instantiate(bytes, {
  env: {
    js_log: (p, l) => console.error('wasm:', dec.decode(new Uint8Array(ex.memory.buffer, p, l))),
    js_now: () => performance.now(),
  },
});
ex = instance.exports;
const out = (n) => dec.decode(new Uint8Array(ex.memory.buffer, ex.out_ptr(), n));
const input = (s) => { const b = enc.encode(s); new Uint8Array(ex.memory.buffer, ex.inbuf(), b.length).set(b); return b.length; };
const cmd = (s) => ex.ui_cmd(input(s));
const fail = (m) => { console.error('SMOKE FAIL:', m); process.exit(1); };

function checkDraw() {
  const n = ex.cmd_len();
  const U = new Uint32Array(ex.memory.buffer, ex.cmd_ptr(), n);
  const sizes = { 1: 1, 2: 2, 6: 4, 5: 4, 7: 4, 8: 7, 9: 1, 10: 6, 11: 4, 12: 4, 13: 2, 14: 7, 15: 10, 16: 3, 17: 0, 18: 3, 19: 1, 20: 1 };
  let i = 0, ops = 0;
  while (i < n) {
    const op = U[i++];
    if (op === 3 || op === 4) { i += 1 + U[i] * 2; } else if (sizes[op] !== undefined) { i += sizes[op]; } else fail(`bad op ${op} at ${i - 1}`);
    ops++;
  }
  if (i !== n) fail('draw buffer misaligned');
  return ops;
}

ex.init(1997);
if (ex.tex_count() !== 19) fail('textures');
// interface art: icon sheet, panel, frame, font, glyph advances
for (let i = 0; i < 4; i++) if (!(ex.ui_img_w(i) > 0 && ex.ui_img_h(i) > 0)) fail('ui art ' + i);
{
  const w = ex.ui_img_w(0), h = ex.ui_img_h(0);
  const px = new Uint8Array(ex.memory.buffer, ex.ui_img_ptr(0), w * h * 4);
  let opaque = 0;
  for (let i = 3; i < px.length; i += 4) if (px[i]) opaque++;
  if (opaque < 20000) fail('icons look empty: ' + opaque);
  const adv = new Uint8Array(ex.memory.buffer, ex.font_adv_ptr(), 95);
  if (adv[33] < 4) fail('font advances');
}
ex.set_pix(4 / 3);
ex.resize(390, 650);
ex.frame(0.016);
if (JSON.parse(out(ex.ui_json())).screen !== 'create') fail('expected create screen');
let baked = 0;
for (const t of ['warrior', 'mage', 'blacksmith', 'woodsman']) {
  ex.init(1997);
  cmd(`create:Smoke:f:2:${t}`);
  for (let k = 0; k < 400; k++) {
    ex.frame(1 / 30);
    while (ex.atlas_next() >= 0) { if (ex.atlas_w() * ex.atlas_h() === 0) fail('empty atlas'); baked++; }
    if (k % 40 === 0) { ex.ptr_down(200 + (k % 120), 560); ex.ptr_up(200 + (k % 120), 560); }
    if (k % 97 === 0) { cmd('war'); cmd('bandage'); cmd('cast:0'); cmd('kick'); cmd('block:1'); cmd('ride'); }
    if (k % 97 === 20) cmd('block:0');
  }
  const hud = JSON.parse(out(ex.hud_json()));
  if (!hud.created || hud.name !== 'Smoke') fail('hud after create');
  for (const w of ['pack', 'skills', 'spells', 'paperdoll', 'journal', 'help', 'map', 'chronicle']) {
    cmd(`open:${w}`);
    const u = JSON.parse(out(ex.ui_json()));
    if (!u.win || !Array.isArray(u.win.rows)) fail(`window ${w}`);
  }
  cmd('close');
  const ops = checkDraw();
  const n = ex.save();
  const save = out(n);
  ex.load(input(save));
  ex.frame(0.016);
  const hud2 = JSON.parse(out(ex.hud_json()));
  if (hud2.name !== 'Smoke') fail('save/load');
  console.log(`${t}: ok, ${ops} draw ops, hp ${hud.hp}/${hud.mhp}, save ${n} bytes`);
}
if (baked === 0) fail('no sprites baked');
// world packs: read one, and refuse to swap packs once the world is built
{
  const pk = (fn, s) => { const b = enc.encode(s); new Uint8Array(ex.memory.buffer, ex.pack_buf(b.length), b.length).set(b); return JSON.parse(out(ex[fn](b.length))); };
  const r = pk('pack_check', '[pack]\nid = "smoke"\nname = "Smoke Wórld"\n[mon.wolf]\nname = "a warg"\n[mon.nope]\nname = "x"\n');
  if (!r.ok || r.pack.name !== 'Smoke Wórld' || r.warnings.length !== 1) fail('pack_check: ' + JSON.stringify(r));
  if (pk('pack_check', '[pack]\nname = "no id"\n').ok) fail('pack_check accepted a pack with no id');
  if (pk('pack_set', '[pack]\nid = "late"\n').ok) fail('pack_set after init');
  if (JSON.parse(out(ex.pack_info())).id !== 'brodilka') fail('pack_info');
}
console.log(`atlases baked: ${baked}`);
console.log('SMOKE OK');
