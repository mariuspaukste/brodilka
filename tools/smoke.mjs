// CI smoke test for the real wasm build: boots the module in Node, plays a
// little, opens every window, validates the draw buffer, and round-trips a save.
import fs from 'node:fs';

const bytes = fs.readFileSync(process.argv[2] || 'dist/brodilka.wasm');
const dec = new TextDecoder(), enc = new TextEncoder();
let ex;
const { instance } = await WebAssembly.instantiate(bytes, {
  env: { js_log: (p, l) => console.error('wasm:', dec.decode(new Uint8Array(ex.memory.buffer, p, l))) },
});
ex = instance.exports;
const out = (n) => dec.decode(new Uint8Array(ex.memory.buffer, ex.out_ptr(), n));
const input = (s) => { const b = enc.encode(s); new Uint8Array(ex.memory.buffer, ex.inbuf(), b.length).set(b); return b.length; };
const cmd = (s) => ex.ui_cmd(input(s));
const fail = (m) => { console.error('SMOKE FAIL:', m); process.exit(1); };

function checkDraw() {
  const n = ex.cmd_len();
  const U = new Uint32Array(ex.memory.buffer, ex.cmd_ptr(), n);
  const sizes = { 1: 1, 2: 2, 6: 4, 5: 4, 7: 4, 8: 7, 9: 1, 10: 6, 11: 4, 12: 4, 13: 2, 14: 7, 15: 10 };
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
if (ex.tex_count() !== 13) fail('textures');
ex.set_pix(4 / 3);
ex.resize(390, 780);
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
    if (k % 97 === 0) { cmd('war'); cmd('bandage'); cmd('cast:0'); }
  }
  const hud = JSON.parse(out(ex.hud_json()));
  if (!hud.created || hud.name !== 'Smoke') fail('hud after create');
  for (const w of ['pack', 'skills', 'spells', 'paperdoll', 'journal', 'help', 'map']) {
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
console.log(`atlases baked: ${baked}`);
console.log('SMOKE OK');
