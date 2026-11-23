// Brodilka host: loads the Rust/WASM core, replays its draw commands on a
// canvas and renders its UI JSON as HTML windows. No game logic lives here.
'use strict';

const WORLD_SEED = 1997;
const SAVE_KEY = 'brodilka.save.v1';
const $ = (id) => document.getElementById(id);
const canvas = $('c');
const txt = $('txt');
const tctx = txt.getContext('2d');
// The screen is a WebGL canvas that palette-quantizes and upscales the
// low-resolution world buffer; 2D canvas is the fallback.
const gl = canvas.getContext('webgl', { antialias: false, preserveDrawingBuffer: true, alpha: false });
const ctx = gl ? null : canvas.getContext('2d');
const low = document.createElement('canvas');
const lctx = low.getContext('2d');
const dec = new TextDecoder();
const enc = new TextEncoder();
let ex = null;
let dpr = 1;

// ------------------------------------------------------------------ draw replay
const colCache = new Map();
function col(u) {
  let s = colCache.get(u);
  if (!s) {
    s = `rgba(${u >>> 24},${(u >>> 16) & 255},${(u >>> 8) & 255},${((u & 255) / 255).toFixed(3)})`;
    colCache.set(u, s);
  }
  return s;
}
function rgbA(u, a) { return `rgba(${u >>> 24},${(u >>> 16) & 255},${(u >>> 8) & 255},${a})`; }

// Replays the command buffer. Text is collected into `texts` (when given) so
// it can be drawn sharp on top of the pixelated world.
function replay(ctx, U, F, S, texts) {
  const n = U.length;
  let i = 0;
  let baseAlpha = 1;
  let inkW = 0, inkC = '#000';
  const ink = () => {
    if (inkW <= 0) return;
    const ss = ctx.strokeStyle, lw = ctx.lineWidth;
    ctx.strokeStyle = inkC; ctx.lineWidth = inkW;
    ctx.stroke();
    ctx.strokeStyle = ss; ctx.lineWidth = lw;
  };
  ctx.globalAlpha = 1;
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  while (i < n) {
    const op = U[i++];
    switch (op) {
      case 1: ctx.fillStyle = col(U[i++]); break;
      case 2: ctx.strokeStyle = col(U[i++]); ctx.lineWidth = F[i++]; break;
      case 3: case 4: {
        const k = U[i++];
        ctx.beginPath();
        ctx.moveTo(F[i], F[i + 1]);
        for (let j = 1; j < k; j++) ctx.lineTo(F[i + j * 2], F[i + j * 2 + 1]);
        i += k * 2;
        if (op === 3) { ctx.closePath(); ctx.fill(); ink(); } else ctx.stroke();
        break;
      }
      case 5: ctx.beginPath(); ctx.ellipse(F[i], F[i + 1], F[i + 2], F[i + 3], 0, 0, Math.PI * 2); ctx.fill(); ink(); i += 4; break;
      case 6: ctx.fillRect(F[i], F[i + 1], F[i + 2], F[i + 3]); i += 4; break;
      case 7: ctx.beginPath(); ctx.moveTo(F[i], F[i + 1]); ctx.lineTo(F[i + 2], F[i + 3]); ctx.stroke(); i += 4; break;
      case 8: {
        const x = F[i], y = F[i + 1], size = F[i + 2], align = U[i + 3], off = U[i + 4], len = U[i + 5], outline = U[i + 6];
        i += 7;
        const s = dec.decode(S.subarray(off, off + len));
        if (texts) { texts.push([s, x, y, size, align, outline, ctx.fillStyle, ctx.globalAlpha]); break; }
        ctx.font = `bold ${size.toFixed(1)}px Palatino, Georgia, serif`;
        ctx.textAlign = align === 0 ? 'center' : 'left';
        ctx.textBaseline = 'middle';
        if ((outline & 255) > 0) {
          const fs = ctx.fillStyle;
          ctx.strokeStyle = col(outline); ctx.lineWidth = Math.max(2, size / 4);
          ctx.strokeText(s, x, y);
          ctx.fillStyle = fs;
        }
        ctx.fillText(s, x, y);
        break;
      }
      case 9: baseAlpha = F[i++]; ctx.globalAlpha = Math.min(1, baseAlpha); break;
      case 10: {
        const x = F[i], y = F[i + 1], r0 = F[i + 2], r1 = F[i + 3], a = F[i + 4], c = U[i + 5];
        i += 6;
        const g = ctx.createRadialGradient(x, y, r0, x, y, r1);
        g.addColorStop(0, rgbA(c, 0));
        g.addColorStop(1, rgbA(c, a));
        const fs = ctx.fillStyle;
        ctx.fillStyle = g;
        ctx.fillRect(-4000, -4000, 12000, 12000);
        ctx.fillStyle = fs;
        break;
      }
      case 13: inkC = col(U[i++]); inkW = F[i++]; break;
      case 15: {
        const cv = atlases.get(U[i]);
        const fr = U[i + 1], fw = U[i + 2], fh = U[i + 3], ax = F[i + 4], ay = F[i + 5], x = F[i + 6], y = F[i + 7], sc = F[i + 8], fl = U[i + 9];
        i += 10;
        if (!cv) break;
        const sx = (fr % 16) * fw, sy = Math.floor(fr / 16) * fh;
        const ga = ctx.globalAlpha;
        if (fl & 2) ctx.globalAlpha = ga * 0.5;
        if (fl & 1) {
          ctx.save(); ctx.translate(x, y); ctx.scale(-1, 1);
          ctx.drawImage(cv, sx, sy, fw, fh, -ax * sc, -ay * sc, fw * sc, fh * sc);
          ctx.restore();
        } else ctx.drawImage(cv, sx, sy, fw, fh, x - ax * sc, y - ay * sc, fw * sc, fh * sc);
        ctx.globalAlpha = ga;
        break;
      }
      case 14: {
        const p = patterns[U[i]];
        if (p) { p.setTransform(new DOMMatrix([F[i + 1], F[i + 2], F[i + 3], F[i + 4], F[i + 5], F[i + 6]])); ctx.fillStyle = p; }
        i += 7;
        break;
      }
      case 11: ctx.beginPath(); ctx.ellipse(F[i], F[i + 1], F[i + 2], F[i + 3], 0, 0, Math.PI * 2); ctx.stroke(); i += 4; break;
      case 12: {
        const x = F[i], y = F[i + 1], r = F[i + 2], c = U[i + 3];
        i += 4;
        const g = ctx.createRadialGradient(x, y, 0, x, y, r);
        const a = Math.min(1, baseAlpha) * 0.5;
        g.addColorStop(0, rgbA(c, a));
        g.addColorStop(1, rgbA(c, 0));
        const fs = ctx.fillStyle, ga = ctx.globalAlpha, op2 = ctx.globalCompositeOperation;
        ctx.globalAlpha = 1;
        ctx.globalCompositeOperation = 'lighter';
        ctx.fillStyle = g;
        ctx.fillRect(x - r, y - r, r * 2, r * 2);
        ctx.globalCompositeOperation = op2;
        ctx.globalAlpha = ga;
        ctx.fillStyle = fs;
        break;
      }
      default:
        console.warn('bad draw op', op, 'at', i - 1);
        return;
    }
  }
  ctx.globalAlpha = 1;
}
// ------------------------------------------------------------------ VGA look
// Textures and the 256-colour palette come from Rust (src/tex.rs).
const patterns = [];
// baked character atlases (id -> canvas), handed over by Rust as they are baked
const atlases = new Map();
function addAtlas(id, w, h, data) {
  const cv = document.createElement('canvas');
  cv.width = w; cv.height = h;
  cv.getContext('2d').putImageData(new ImageData(new Uint8ClampedArray(data), w, h), 0, 0);
  atlases.set(id, cv);
}
window.brodilkaAddAtlas = addAtlas;
function pollAtlases() {
  for (;;) {
    const id = ex.atlas_next();
    if (id < 0) break;
    const w = ex.atlas_w(), h = ex.atlas_h();
    addAtlas(id, w, h, new Uint8Array(mem(), ex.atlas_ptr(), w * h * 4).slice());
  }
  for (;;) { const id = ex.atlas_evicted(); if (id < 0) break; atlases.delete(id); }
}
let lut = null;
function initArt(texs, pal) {
  patterns.length = 0;
  for (const t of texs) {
    const cv = document.createElement('canvas');
    cv.width = t.w; cv.height = t.h;
    cv.getContext('2d').putImageData(new ImageData(new Uint8ClampedArray(t.data), t.w, t.h), 0, 0);
    patterns.push(lctx.createPattern(cv, 'repeat'));
  }
  // 32x32x32 lookup from RGB to the nearest palette colour, laid out 1024x32
  lut = new Uint8Array(1024 * 32 * 4);
  for (let r = 0; r < 32; r++) for (let g = 0; g < 32; g++) for (let b = 0; b < 32; b++) {
    const R = r * 8 + 4, G = g * 8 + 4, B = b * 8 + 4;
    let best = 0, bd = 1e9;
    for (let k = 0; k < 256; k++) {
      const dr = R - pal[k * 3], dg = G - pal[k * 3 + 1], db = B - pal[k * 3 + 2];
      const d = dr * dr * 3 + dg * dg * 4 + db * db * 2;
      if (d < bd) { bd = d; best = k; }
    }
    const o = (r * 1024 + b * 32 + g) * 4;
    lut[o] = pal[best * 3]; lut[o + 1] = pal[best * 3 + 1]; lut[o + 2] = pal[best * 3 + 2]; lut[o + 3] = 255;
  }
  if (gl) glSetup();
}
window.brodilkaInitArt = initArt;
let glProg = null, glSrc = null, glLut = null, glBayer = null;
function glShader(type, src) {
  const sh = gl.createShader(type);
  gl.shaderSource(sh, src);
  gl.compileShader(sh);
  if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(sh));
  return sh;
}
function glTex(filter, wrap) {
  const t = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D, t);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, filter);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, filter);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, wrap);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, wrap);
  return t;
}
function glSetup() {
  const vs = 'attribute vec2 p; varying vec2 uv; void main(){ uv = vec2(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5); gl_Position = vec4(p, 0.0, 1.0); }';
  // ordered dither, then snap to the 256-colour palette through the LUT
  const fs = `precision mediump float;
    uniform sampler2D src; uniform sampler2D lut; uniform sampler2D bay; uniform vec2 size; varying vec2 uv;
    void main(){
      vec2 px = floor(uv * size);
      vec3 c = texture2D(src, (px + 0.5) / size).rgb * 255.0;
      float o = (texture2D(bay, (px + 0.5) / 4.0).r * 255.0 / 16.0 / 16.0 - 0.47) * 18.0;
      vec3 q = floor(clamp(c + o, 0.0, 255.0) / 8.0);
      gl_FragColor = texture2D(lut, vec2((q.b * 32.0 + q.g + 0.5) / 1024.0, (q.r + 0.5) / 32.0));
    }`;
  glProg = gl.createProgram();
  gl.attachShader(glProg, glShader(gl.VERTEX_SHADER, vs));
  gl.attachShader(glProg, glShader(gl.FRAGMENT_SHADER, fs));
  gl.linkProgram(glProg);
  gl.useProgram(glProg);
  const buf = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, buf);
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);
  const loc = gl.getAttribLocation(glProg, 'p');
  gl.enableVertexAttribArray(loc);
  gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
  glSrc = glTex(gl.NEAREST, gl.CLAMP_TO_EDGE);
  glLut = glTex(gl.NEAREST, gl.CLAMP_TO_EDGE);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 1024, 32, 0, gl.RGBA, gl.UNSIGNED_BYTE, lut);
  glBayer = glTex(gl.NEAREST, gl.REPEAT);
  const bay = new Uint8Array([0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5].map((v) => v * 16));
  gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.LUMINANCE, 4, 4, 0, gl.LUMINANCE, gl.UNSIGNED_BYTE, bay);
  gl.uniform1i(gl.getUniformLocation(glProg, 'src'), 0);
  gl.uniform1i(gl.getUniformLocation(glProg, 'lut'), 1);
  gl.uniform1i(gl.getUniformLocation(glProg, 'bay'), 2);
}
function glPresent(lw, lh) {
  gl.viewport(0, 0, canvas.width, canvas.height);
  gl.activeTexture(gl.TEXTURE0);
  gl.bindTexture(gl.TEXTURE_2D, glSrc);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, low);
  gl.activeTexture(gl.TEXTURE1);
  gl.bindTexture(gl.TEXTURE_2D, glLut);
  gl.activeTexture(gl.TEXTURE2);
  gl.bindTexture(gl.TEXTURE_2D, glBayer);
  gl.uniform2f(gl.getUniformLocation(glProg, 'size'), lw, lh);
  gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
}
// overhead text is drawn at art resolution with a small pixel-ish font
function drawTexts(c, texts) {
  c.setTransform(1, 0, 0, 1, 0, 0);
  c.textBaseline = 'middle';
  c.lineJoin = 'round';
  for (const [s, x, y, size, align, outline, fill, alpha] of texts) {
    const px = Math.round(Math.max(8, Math.min(10, size / pix * 0.7)));
    c.globalAlpha = alpha;
    c.font = `bold ${px}px "Courier New", Courier, monospace`;
    c.textAlign = align === 0 ? 'center' : 'left';
    const X = Math.round(x / pix), Y = Math.round(y / pix);
    if ((outline & 255) > 0) { c.strokeStyle = col(outline); c.lineWidth = 2; c.strokeText(s, X, Y); }
    c.fillStyle = fill;
    c.fillText(s, X, Y);
  }
  c.globalAlpha = 1;
}
// Low-resolution "VGA" buffer (~320 pixels across), scaled up with nearest-neighbour.
let pix = 1, artK = 1;
function present(U, F, S) {
  const lw = Math.ceil(canvas.width / artK), lh = Math.ceil(canvas.height / artK);
  if (low.width !== lw || low.height !== lh) { low.width = lw; low.height = lh; }
  if (txt.width !== lw || txt.height !== lh) { txt.width = lw; txt.height = lh; }
  lctx.setTransform(1 / pix, 0, 0, 1 / pix, 0, 0);
  lctx.imageSmoothingEnabled = false;
  const texts = [];
  replay(lctx, U, F, S, texts);
  if (gl && glProg) glPresent(lw, lh);
  else if (ctx) { ctx.setTransform(1, 0, 0, 1, 0, 0); ctx.imageSmoothingEnabled = false; ctx.drawImage(low, 0, 0, lw, lh, 0, 0, lw * artK, lh * artK); }
  tctx.setTransform(1, 0, 0, 1, 0, 0);
  tctx.clearRect(0, 0, lw, lh);
  drawTexts(tctx, texts);
}
window.brodilkaReplay = replay;
window.brodilkaPresent = present;

// ------------------------------------------------------------------ wasm bridge
function mem() { return ex.memory.buffer; }
function readStr(ptr, len) { return dec.decode(new Uint8Array(mem(), ptr, len)); }
function writeIn(s) {
  const b = enc.encode(s);
  new Uint8Array(mem(), ex.inbuf(), b.length).set(b);
  return b.length;
}
function outStr(len) { return readStr(ex.out_ptr(), len); }
function cmd(s) {
  ex.ui_cmd(writeIn(s));
  refreshUI(true);
}
window.brodilkaCmd = cmd;

// ------------------------------------------------------------------ sizing
function resize() {
  dpr = Math.min(window.devicePixelRatio || 1, 2);
  const w = window.innerWidth, h = window.innerHeight;
  canvas.width = Math.round(w * dpr);
  canvas.height = Math.round(h * dpr);
  artK = Math.max(1, Math.round(canvas.width / 320));
  pix = artK / dpr;
  if (ex) { ex.resize(w, h); ex.set_pix(pix); }
}
window.addEventListener('resize', resize);

// ------------------------------------------------------------------ minimap / radar
const radar = $('radar');
const rctx = radar.getContext('2d');
const mapCanvas = document.createElement('canvas');
function buildMinimap() {
  const W = ex.map_w();
  mapCanvas.width = W; mapCanvas.height = W;
  const data = new Uint8ClampedArray(mem(), ex.mini_ptr(), W * W * 4).slice();
  mapCanvas.getContext('2d').putImageData(new ImageData(data, W, W), 0, 0);
}
function drawRadar() {
  const R = radar.width, s = 2.6, px = ex.px(), py = ex.py();
  rctx.setTransform(1, 0, 0, 1, 0, 0);
  rctx.fillStyle = '#0a1a2a';
  rctx.fillRect(0, 0, R, R);
  // isometric projection matching the game view
  const a = s, b = s / 2, c = -s, d = s / 2;
  rctx.setTransform(a, b, c, d, R / 2 - (a * px + c * py), R / 2 - (b * px + d * py));
  rctx.imageSmoothingEnabled = false;
  rctx.drawImage(mapCanvas, 0, 0);
  const dots = outStr(ex.radar_dots()).split(';');
  for (const dstr of dots) {
    if (!dstr) continue;
    const k = dstr[0];
    const [x, y] = dstr.slice(1).split(',').map(Number);
    rctx.fillStyle = k === 'm' ? '#ff3a2a' : k === 'n' ? '#5aa0ff' : '#e8e0c0';
    rctx.fillRect(x - 0.5, y - 0.5, 1.2, 1.2);
  }
  const cx = ex.corpse_x();
  if (cx >= 0) { rctx.fillStyle = '#ffd040'; rctx.fillRect(cx - 1, ex.corpse_y() - 1, 2.2, 2.2); }
  rctx.setTransform(1, 0, 0, 1, 0, 0);
  rctx.fillStyle = '#fff';
  rctx.beginPath(); rctx.arc(R / 2, R / 2, 4, 0, Math.PI * 2); rctx.fill();
  rctx.strokeStyle = '#000'; rctx.lineWidth = 1.5; rctx.stroke();
}
radar.addEventListener('click', () => cmd('toggle:map'));

// ------------------------------------------------------------------ HUD
let hudVer = -1, uiVer = -1, created = false, lastHud = null;
const shownJ = new Set();
function refreshUI(force) {
  if (!ex) return;
  const hv = ex.hud_ver(), uv = ex.ui_ver();
  if (force || hv !== hudVer) { hudVer = hv; renderHud(JSON.parse(outStr(ex.hud_json()))); }
  if (force || uv !== uiVer) { uiVer = uv; renderUI(JSON.parse(outStr(ex.ui_json()))); }
}
function renderHud(h) {
  lastHud = h;
  created = h.created;
  $('hud').classList.toggle('hidden', !h.created);
  if (!h.created) return;
  $('pname').textContent = h.ghost ? `${h.name} (ghost)` : h.name;
  $('hpb').style.width = `${(100 * h.hp / Math.max(1, h.mhp)).toFixed(1)}%`;
  $('hpt').textContent = `${h.hp} / ${h.mhp}`;
  $('mpb').style.width = `${(100 * h.mana / Math.max(1, h.mmana)).toFixed(1)}%`;
  $('mpt').textContent = `${h.mana} / ${h.mmana}`;
  $('gold').textContent = `🪙 ${h.gold}`;
  const bw = $('bwar');
  bw.classList.toggle('on', h.war);
  bw.querySelector('small').textContent = h.war ? 'War' : 'Peace';
  $('combat').classList.toggle('hidden', !h.war || h.ghost);
  $('bblock').classList.toggle('on', h.blocking);
  $('bband').querySelector('small').textContent = h.band;
  $('bpot').querySelector('small').textContent = h.pot;
  $('blast').querySelector('small').textContent = h.last ? h.last.split(' ')[0] : 'Last';
  if (h.tgt) {
    $('target').classList.remove('hidden');
    $('tname').textContent = h.tgt.name;
    $('tb').style.width = `${(100 * h.tgt.hp / Math.max(1, h.tgt.mhp)).toFixed(1)}%`;
  } else $('target').classList.add('hidden');
  $('act').classList.toggle('hidden', !h.act);
  $('act').textContent = h.act;
  $('banner').classList.toggle('hidden', !h.targeting);
  $('bannertext').textContent = h.targeting ? `🎯 ${h.targeting}: tap a target` : '';
  canvas.style.filter = h.ghost ? 'grayscale(1) brightness(0.9)' : '';
  const j = $('journal');
  for (const [id, text, color] of h.j) {
    if (shownJ.has(id)) continue;
    shownJ.add(id);
    const d = document.createElement('div');
    d.textContent = text;
    d.style.color = color;
    j.appendChild(d);
    setTimeout(() => d.remove(), 9000);
    while (j.children.length > 7) j.firstChild.remove();
  }
}

// ------------------------------------------------------------------ windows
const win = $('win');
function el(tag, cls, text) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text !== undefined) e.textContent = text;
  return e;
}
function btn(label, c) {
  const b = el('button', '', label);
  b.addEventListener('click', (ev) => { ev.stopPropagation(); cmd(c); });
  return b;
}
let lastWinKey = '';
function renderUI(u) {
  $('create').classList.toggle('hidden', u.screen !== 'create');
  if (u.screen === 'create') { setupCreate(u); win.classList.add('hidden'); return; }
  const w = u.win;
  if (!w) { win.classList.add('hidden'); lastWinKey = ''; return; }
  const scrollKey = w.id + w.title;
  const oldRows = win.querySelector('.rows');
  const scroll = scrollKey === lastWinKey && oldRows ? oldRows.scrollTop : 0;
  lastWinKey = scrollKey;
  win.innerHTML = '';
  win.classList.remove('hidden');
  const t = el('div', 'wt');
  t.appendChild(el('span', '', w.title));
  const x = el('button', 'x', '✕');
  x.addEventListener('click', () => cmd('close'));
  t.appendChild(x);
  win.appendChild(t);
  if (w.tabs.length) {
    const tabs = el('div', 'tabs');
    for (const [l, c, on] of w.tabs) { const b = btn(l, c); if (on) b.classList.add('on'); tabs.appendChild(b); }
    win.appendChild(tabs);
  }
  if (w.head.length) {
    const hd = el('div', 'head');
    for (const line of w.head) hd.appendChild(el('div', '', line));
    win.appendChild(hd);
  }
  if (w.rows.length) {
    const rows = el('div', 'rows');
    for (const r of w.rows) {
      const row = el('div', 'r' + (r.dim ? ' dim' : ''));
      row.appendChild(el('div', 'ic', r.i));
      const tx = el('div', 'tx');
      tx.appendChild(el('div', '', r.t));
      if (r.s) tx.appendChild(el('div', 's', r.s));
      if (r.bar !== undefined) { const pb = el('div', 'pb'); const i = el('i'); i.style.width = `${(r.bar * 100).toFixed(1)}%`; pb.appendChild(i); tx.appendChild(pb); }
      row.appendChild(tx);
      for (const [l, c] of r.b) row.appendChild(btn(l, c));
      rows.appendChild(row);
    }
    win.appendChild(rows);
    rows.scrollTop = scroll;
  }
  if (w.foot.length) {
    const f = el('div', 'foot');
    for (const [l, c] of w.foot) f.appendChild(btn(l, c));
    win.appendChild(f);
  }
}

// ------------------------------------------------------------------ character creation
let createReady = false, gender = 'm', hair = 0, tmpl = 'warrior';
function setupCreate(u) {
  if (createReady) return;
  createReady = true;
  const save = localStorage.getItem(SAVE_KEY);
  if (save) {
    const m = save.match(/^name=(.*)$/m);
    $('cont').classList.remove('hidden');
    $('contname').textContent = m ? m[1] : 'thy hero';
    $('bcontinue').onclick = () => {
      const ok = ex.load(writeIn(save));
      atlases.clear();
      buildMinimap();
      resize();
      if (!ok) alert('Could not read the save.');
      refreshUI(true);
    };
  }
  const hairs = $('hairs');
  u.hair.forEach((h, k) => {
    const i = el('i' + '');
    i.style.background = h;
    if (k === 0) i.classList.add('sel');
    i.onclick = () => { hair = k; [...hairs.children].forEach((c, j) => c.classList.toggle('sel', j === k)); };
    hairs.appendChild(i);
  });
  document.querySelectorAll('.seg').forEach((b) => b.onclick = () => {
    gender = b.dataset.g;
    document.querySelectorAll('.seg').forEach((c) => c.classList.toggle('sel', c === b));
  });
  const box = $('tmpls');
  const all = [...u.templates, ['custom', 'Custom', 'Pick any three skills.']];
  for (const [id, name, desc] of all) {
    const b = el('button');
    b.appendChild(el('b', '', name));
    b.appendChild(document.createTextNode(desc));
    if (id === tmpl) b.classList.add('sel');
    b.onclick = () => {
      tmpl = id;
      [...box.children].forEach((c) => c.classList.toggle('sel', c === b));
      $('custom').classList.toggle('hidden', id !== 'custom');
    };
    box.appendChild(b);
  }
  ['cs1', 'cs2', 'cs3'].forEach((s, k) => {
    const sel = $(s);
    for (const [id, name] of u.skills) { const o = el('option', '', name); o.value = id; sel.appendChild(o); }
    sel.value = [0, 3, 7][k];
  });
  $('bstart').onclick = () => {
    if (save && !confirm('Start a new character? Thy saved hero will be replaced.')) return;
    const name = ($('cname').value || 'Stranger').replace(/[:\n]/g, '').slice(0, 16);
    let c = `create:${name}:${gender}:${hair}:${tmpl}`;
    if (tmpl === 'custom') c += `:${$('cs1').value}:${$('cs2').value}:${$('cs3').value}`;
    cmd(c);
    saveGame();
  };
}

// ------------------------------------------------------------------ input
const pointers = new Map();
let pinchD = 0;
function rel(e) { return [e.clientX, e.clientY]; }
canvas.addEventListener('pointerdown', (e) => {
  canvas.setPointerCapture(e.pointerId);
  pointers.set(e.pointerId, rel(e));
  if (pointers.size === 1) ex.ptr_down(...rel(e));
  else { ex.ptr_cancel(); pinchD = pinchDist(); }
});
canvas.addEventListener('pointermove', (e) => {
  if (!pointers.has(e.pointerId)) return;
  pointers.set(e.pointerId, rel(e));
  if (pointers.size === 1) ex.ptr_move(...rel(e));
  else if (pointers.size === 2) {
    const d = pinchDist();
    if (pinchD > 0 && d > 0) ex.zoom(d / pinchD);
    pinchD = d;
  }
});
function up(e) {
  if (!pointers.has(e.pointerId)) return;
  const wasSingle = pointers.size === 1;
  pointers.delete(e.pointerId);
  if (wasSingle) { ex.ptr_up(...rel(e)); refreshUI(); }
}
canvas.addEventListener('pointerup', up);
canvas.addEventListener('pointercancel', (e) => { pointers.delete(e.pointerId); ex.ptr_cancel(); });
canvas.addEventListener('contextmenu', (e) => e.preventDefault());
canvas.addEventListener('wheel', (e) => { e.preventDefault(); ex.zoom(e.deltaY < 0 ? 1.1 : 1 / 1.1); }, { passive: false });
function pinchDist() {
  const p = [...pointers.values()];
  return p.length < 2 ? 0 : Math.hypot(p[0][0] - p[1][0], p[0][1] - p[1][1]);
}
document.querySelectorAll('[data-cmd]').forEach((b) => b.addEventListener('click', (e) => { e.stopPropagation(); cmd(b.dataset.cmd); }));

// combat buttons: block is held, kick is tapped
const bblock = $('bblock');
let blockHeld = false;
function setBlock(on) { if (blockHeld === on) return; blockHeld = on; cmd(on ? 'block:1' : 'block:0'); }
bblock.addEventListener('pointerdown', (e) => { e.preventDefault(); bblock.setPointerCapture(e.pointerId); setBlock(true); });
['pointerup', 'pointercancel', 'lostpointercapture'].forEach((ev) => bblock.addEventListener(ev, () => setBlock(false)));
$('bkick').addEventListener('pointerdown', (e) => { e.preventDefault(); cmd('kick'); });

let keyBits = 0;
const KEYMAP = { ArrowUp: 1, KeyW: 1, ArrowRight: 2, KeyD: 2, ArrowDown: 4, KeyS: 4, ArrowLeft: 8, KeyA: 8 };
window.addEventListener('keydown', (e) => {
  if (!ex || !created || e.target.tagName === 'INPUT') return;
  if (KEYMAP[e.code]) { keyBits |= KEYMAP[e.code]; ex.keys(keyBits); e.preventDefault(); return; }
  const k = e.code;
  if (k === 'Tab') { e.preventDefault(); cmd('war'); }
  else if (k === 'KeyB') cmd('bandage');
  else if (k === 'KeyP' || k === 'KeyI') cmd('toggle:pack');
  else if (k === 'KeyK') cmd('toggle:skills');
  else if (k === 'KeyM') cmd('toggle:map');
  else if (k === 'KeyJ') cmd('toggle:journal');
  else if (k === 'KeyH') cmd('potion');
  else if (k === 'KeyR') cmd('lastspell');
  else if (k === 'Escape') cmd('escape');
  else if (k === 'Space') { e.preventDefault(); if (!e.repeat) setBlock(true); }
  else if (k === 'KeyF') cmd('kick');
  else if (/^Digit[1-9]$/.test(k)) cmd(`cast:${+k.slice(5) - 1}`);
});
window.addEventListener('keyup', (e) => {
  if (e.code === 'Space') setBlock(false);
  if (KEYMAP[e.code]) { keyBits &= ~KEYMAP[e.code]; if (ex) ex.keys(keyBits); }
});
window.addEventListener('blur', () => { keyBits = 0; if (ex) { ex.keys(0); setBlock(false); } });

// ------------------------------------------------------------------ saving
function saveGame() {
  if (!ex || !created) return;
  const n = ex.save();
  if (n > 0) try { localStorage.setItem(SAVE_KEY, outStr(n)); } catch (_) { /* storage may be unavailable */ }
}
setInterval(saveGame, 15000);
document.addEventListener('visibilitychange', () => { if (document.hidden) saveGame(); });
window.addEventListener('pagehide', saveGame);

// ------------------------------------------------------------------ main loop
let last = performance.now(), frameNo = 0;
function loop(now) {
  const dt = Math.min(0.1, (now - last) / 1000);
  last = now;
  ex.frame(dt);
  pollAtlases();
  const U = new Uint32Array(mem(), ex.cmd_ptr(), ex.cmd_len());
  const F = new Float32Array(mem(), ex.cmd_ptr(), ex.cmd_len());
  const S = new Uint8Array(mem(), ex.str_ptr(), ex.str_len());
  present(U, F, S);
  if (created && (frameNo++ % 3 === 0)) drawRadar();
  refreshUI();
  requestAnimationFrame(loop);
}

async function boot() {
  const imports = { env: { js_log: (p, l) => console.error(readStr(p, l)) } };
  const res = await fetch('brodilka.wasm');
  const bytes = await res.arrayBuffer();
  const { instance } = await WebAssembly.instantiate(bytes, imports);
  ex = instance.exports;
  ex.init(WORLD_SEED);
  const texs = [];
  for (let k = 0; k < ex.tex_count(); k++) {
    const w = ex.tex_w(k), h = ex.tex_h(k);
    texs.push({ w, h, data: new Uint8Array(mem(), ex.tex_ptr(k), w * h * 4).slice() });
  }
  initArt(texs, new Uint8Array(mem(), ex.pal_ptr(), 768).slice());
  resize();
  buildMinimap();
  $('loading').classList.add('hidden');
  refreshUI(true);
  requestAnimationFrame((t) => { last = t; loop(t); });
}
boot().catch((e) => { $('loading').textContent = 'Failed to load the realm: ' + e; console.error(e); });
