// Brodilka host: loads the Rust/WASM core, replays its draw commands on a
// canvas and renders its UI JSON as HTML windows. No game logic lives here.
'use strict';

const WORLD_SEED = 1997;
// Pagan's 320×200 filled a 4:3 screen, so its pixels stood 20% taller than
// wide; the world is drawn squeezed and stretched back by this much.
const ASPECT = 1.2;
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
//
// Zoomed out, a frame holds thousands of ground diamonds and fringes, so runs
// of opaque, un-inked polygons that share one fill are gathered into a single
// path and filled once; a pattern fill that is already set isn't set again.
const patM = new DOMMatrix();
// ground chunks drawn by Rust once and put down every frame (id -> canvas),
// and freed ones kept for reuse
const chunks = new Map();
const spareChunks = [];
function replay(ctx, U, F, S, texts) {
  const n = U.length;
  let i = 0;
  let baseAlpha = 1;
  let inkW = 0, inkC = '#000';
  // fill batching: is the current fill opaque, is a path being gathered
  let opaque = false, batch = false;
  // the pattern fill last set, so a repeat can be skipped
  let pTex = -1, pA = 0, pB = 0, pC = 0, pD = 0, pE = 0, pF = 0;
  const flush = () => { ctx.fill(); batch = false; };
  const ink = () => {
    if (inkW <= 0) return;
    const ss = ctx.strokeStyle, lw = ctx.lineWidth;
    ctx.strokeStyle = inkC; ctx.lineWidth = inkW;
    ctx.stroke();
    ctx.strokeStyle = ss; ctx.lineWidth = lw;
  };
  const base = ctx.getTransform();
  const outer = []; // while drawing into a chunk: the frame's own state
  ctx.globalAlpha = 1;
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  while (i < n) {
    const op = U[i++];
    if (batch && op !== 3 && op !== 14) flush();
    switch (op) {
      case 1: ctx.fillStyle = col(U[i]); opaque = (U[i++] & 255) === 255; pTex = -1; break;
      case 2: ctx.strokeStyle = col(U[i++]); ctx.lineWidth = F[i++]; break;
      case 3: {
        const k = U[i++];
        if (inkW <= 0 && opaque && baseAlpha >= 1) {
          // one subpath of a shared fill; every subpath wound the same way, so
          // where they overlap the nonzero rule fills rather than cuts a hole
          if (!batch) { ctx.beginPath(); batch = true; }
          let a2 = 0;
          for (let j = 0; j < k; j++) {
            const q = i + j * 2, r = i + ((j + 1) % k) * 2;
            a2 += F[q] * F[r + 1] - F[r] * F[q + 1];
          }
          if (a2 >= 0) {
            ctx.moveTo(F[i], F[i + 1]);
            for (let j = 1; j < k; j++) ctx.lineTo(F[i + j * 2], F[i + j * 2 + 1]);
          } else {
            ctx.moveTo(F[i + (k - 1) * 2], F[i + (k - 1) * 2 + 1]);
            for (let j = k - 2; j >= 0; j--) ctx.lineTo(F[i + j * 2], F[i + j * 2 + 1]);
          }
          ctx.closePath();
          i += k * 2;
          break;
        }
        if (batch) flush();
        ctx.beginPath();
        ctx.moveTo(F[i], F[i + 1]);
        for (let j = 1; j < k; j++) ctx.lineTo(F[i + j * 2], F[i + j * 2 + 1]);
        i += k * 2;
        ctx.closePath(); ctx.fill(); ink();
        break;
      }
      case 4: {
        const k = U[i++];
        ctx.beginPath();
        ctx.moveTo(F[i], F[i + 1]);
        for (let j = 1; j < k; j++) ctx.lineTo(F[i + j * 2], F[i + j * 2 + 1]);
        i += k * 2;
        ctx.stroke();
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
          // mirrored about x: the base transform, moved to x,y and flipped
          ctx.setTransform(-base.a, -base.b, base.c, base.d,
            base.a * x + base.c * y + base.e, base.b * x + base.d * y + base.f);
          ctx.drawImage(cv, sx, sy, fw, fh, -ax * sc, -ay * sc, fw * sc, fh * sc);
          ctx.setTransform(base);
        } else ctx.drawImage(cv, sx, sy, fw, fh, x - ax * sc, y - ay * sc, fw * sc, fh * sc);
        ctx.globalAlpha = ga;
        break;
      }
      case 14: {
        const t = U[i], a = F[i + 1], b = F[i + 2], c = F[i + 3], d = F[i + 4], e = F[i + 5], f = F[i + 6];
        i += 7;
        if (t === pTex && a === pA && b === pB && c === pC && d === pD && e === pE && f === pF) break;
        if (batch) flush();
        const p = patterns[t];
        if (p) {
          patM.a = a; patM.b = b; patM.c = c; patM.d = d; patM.e = e; patM.f = f;
          p.setTransform(patM);
          ctx.fillStyle = p;
          opaque = true; // the terrain textures have no see-through pixels
          pTex = t; pA = a; pB = b; pC = c; pD = d; pE = e; pF = f;
        }
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
      case 16: {
        const id = U[i], w = U[i + 1], h = U[i + 2];
        i += 3;
        let cv = chunks.get(id);
        if (!cv) { cv = spareChunks.pop() || document.createElement('canvas'); chunks.set(id, cv); }
        // Rust paints every pixel of a chunk, so a reused canvas needn't be cleared
        if (cv.width !== w || cv.height !== h) { cv.width = w; cv.height = h; }
        outer.push([ctx, baseAlpha, inkW, inkC]);
        ctx = cv.getContext('2d');
        ctx.setTransform(base);
        ctx.imageSmoothingEnabled = false;
        ctx.globalAlpha = 1;
        ctx.lineCap = 'round';
        ctx.lineJoin = 'round';
        baseAlpha = 1; inkW = 0; opaque = false; pTex = -1;
        break;
      }
      case 17:
        if (outer.length) {
          ctx.globalCompositeOperation = 'source-over';
          [ctx, baseAlpha, inkW, inkC] = outer.pop();
          opaque = false; pTex = -1;
        }
        break;
      case 18: {
        const cv = chunks.get(U[i]);
        if (cv) ctx.drawImage(cv, F[i + 1], F[i + 2], cv.width / base.a, cv.height / base.d);
        i += 3;
        break;
      }
      case 19: {
        const cv = chunks.get(U[i++]);
        if (cv) { chunks.delete(U[i - 1]); if (spareChunks.length < 64) spareChunks.push(cv); }
        break;
      }
      case 20: ctx.globalCompositeOperation = U[i++] ? 'destination-out' : 'source-over'; break;
      default:
        console.warn('bad draw op', op, 'at', i - 1);
        if (batch) flush();
        return;
    }
  }
  if (batch) flush();
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
  const vs = 'attribute vec2 p; void main(){ gl_Position = vec4(p, 0.0, 1.0); }';
  // ordered dither, then snap to the 256-colour palette through the LUT
  // Rows are 1.2× taller than columns, so they don't land on whole screen
  // pixels: the last screen pixel of a row blends into the next one.
  // Each screen pixel finds its art pixel from gl_FragCoord in exact steps of
  // `k` (artK across, artK x ASPECT down), the same grid the 2D path and the
  // text overlay use. highp matters: iPhone and iPad GPUs really run mediump
  // as 16-bit floats, too coarse to place a pixel on a 2000+ pixel wide
  // screen, so edges wandered and art pixels came out 6, 7 or 8 wide.
  const fs = `#ifdef GL_FRAGMENT_PRECISION_HIGH
    precision highp float;
    #else
    precision mediump float;
    #endif
    uniform sampler2D src; uniform sampler2D lut; uniform sampler2D bay; uniform vec2 size; uniform vec2 k; uniform float top;
    vec3 quant(vec2 px){
      vec3 c = texture2D(src, (px + 0.5) / size).rgb * 255.0;
      float o = (texture2D(bay, (px + 0.5) / 4.0).r * 255.0 / 16.0 / 16.0 - 0.47) * 18.0;
      vec3 q = floor(clamp(c + o, 0.0, 255.0) / 8.0);
      return texture2D(lut, vec2((q.b * 32.0 + q.g + 0.5) / 1024.0, (q.r + 0.5) / 32.0)).rgb;
    }
    void main(){
      vec2 t = vec2(gl_FragCoord.x, top - gl_FragCoord.y) / k;
      vec2 px = floor(t);
      float d = (fract(t.y) - 0.5) * k.y;
      float edge = k.y * 0.5 - 0.5;
      vec3 c = quant(px);
      if (d > edge) c = mix(c, quant(px + vec2(0.0, 1.0)), clamp((d - edge) * 0.5, 0.0, 0.5));
      else if (d < -edge) c = mix(c, quant(px - vec2(0.0, 1.0)), clamp((-edge - d) * 0.5, 0.0, 0.5));
      gl_FragColor = vec4(c, 1.0);
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
  gl.uniform2f(gl.getUniformLocation(glProg, 'k'), artK, artK * ASPECT);
  gl.uniform1f(gl.getUniformLocation(glProg, 'top'), canvas.height);
  gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
}
// overhead text is drawn at art resolution with a small pixel-ish font
// ------------------------------------------------------------------ interface art from Rust
let font = null; // { img, adv, tints: Map }
function uiImg(i) {
  const w = ex.ui_img_w(i), h = ex.ui_img_h(i);
  const c = document.createElement('canvas');
  c.width = w; c.height = h;
  c.getContext('2d').putImageData(new ImageData(new Uint8ClampedArray(mem(), ex.ui_img_ptr(i), w * h * 4).slice(), w, h), 0, 0);
  return c;
}
function initUiArt() {
  const root = document.documentElement.style;
  const icons = uiImg(0);
  root.setProperty('--icons', `url(${icons.toDataURL()})`);
  root.setProperty('--icons-w', `${icons.width}px`);
  root.setProperty('--icons-h', `${icons.height}px`);
  root.setProperty('--panel', `url(${uiImg(1).toDataURL()})`);
  root.setProperty('--frame', `url(${uiImg(2).toDataURL()})`);
  document.body.classList.add('painted');
  font = { img: uiImg(3), adv: new Uint8Array(mem(), ex.font_adv_ptr(), 95).slice(), tints: new Map() };
  // toolbar icons replace the emoji, in toolbar order
  document.querySelectorAll('#bar button').forEach((b, k) => {
    const sp = b.querySelector('span');
    if (sp) { sp.textContent = ''; sp.appendChild(icon(k)); }
  });
}
function icon(k) {
  const i = document.createElement('i');
  i.className = 'ico';
  i.style.backgroundPosition = `${-(k % 16) * 48}px ${-Math.floor(k / 16) * 48}px`;
  return i;
}
function fontTint(color) {
  let t = font.tints.get(color);
  if (!t) {
    t = document.createElement('canvas');
    t.width = font.img.width; t.height = font.img.height;
    const g = t.getContext('2d');
    g.drawImage(font.img, 0, 0);
    g.globalCompositeOperation = 'source-in';
    g.fillStyle = color;
    g.fillRect(0, 0, t.width, t.height);
    font.tints.set(color, t);
  }
  return t;
}
function fontWidth(s) {
  let w = 0;
  for (let i = 0; i < s.length; i++) {
    const k = s.charCodeAt(i) - 32;
    w += font.adv[k >= 0 && k < 95 ? k : 31];
  }
  return Math.max(0, w - 1);
}
function fontDraw(c, s, x, y, color) {
  const t = fontTint(color);
  for (let i = 0; i < s.length; i++) {
    let k = s.charCodeAt(i) - 32;
    if (k < 0 || k >= 95) k = 31;
    c.drawImage(t, k * 6, 0, 6, 9, x, y, 6, 9);
    x += font.adv[k];
  }
}
function drawTexts(c, texts) {
  c.setTransform(1, 0, 0, 1, 0, 0);
  if (font) {
    for (const [s, x, y, size, align, outline, fill, alpha] of texts) {
      c.globalAlpha = alpha;
      const w = fontWidth(s);
      const X = Math.round(x / pix - (align === 0 ? w / 2 : 0)), Y = Math.round(y / pix) - 4;
      if ((outline & 255) > 0) {
        const o = col(outline);
        for (const [dx, dy] of [[-1, 0], [1, 0], [0, -1], [0, 1], [1, 1]]) fontDraw(c, s, X + dx, Y + dy, o);
      }
      fontDraw(c, s, X, Y, fill);
    }
    c.globalAlpha = 1;
    return;
  }
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
  const lw = Math.ceil(canvas.width / artK), lh = Math.ceil(canvas.height / (artK * ASPECT));
  if (low.width !== lw || low.height !== lh) { low.width = lw; low.height = lh; }
  const tkey = `${lw}x${lh}x${artK}x${dpr}`;
  if (txt.dataset.k !== tkey) {
    txt.dataset.k = tkey;
    txt.width = lw; txt.height = lh;
    // the same art grid as the world (it may run a few pixels off the edge)
    txt.style.width = `${(lw * artK) / dpr}px`;
    txt.style.height = `${(lh * artK * ASPECT) / dpr}px`;
  }
  lctx.setTransform(1 / pix, 0, 0, 1 / pix, 0, 0);
  lctx.imageSmoothingEnabled = false;
  const texts = [];
  replay(lctx, U, F, S, texts);
  if (gl && glProg) glPresent(lw, lh);
  else if (ctx) { ctx.setTransform(1, 0, 0, 1, 0, 0); ctx.imageSmoothingEnabled = false; ctx.drawImage(low, 0, 0, lw, lh, 0, 0, lw * artK, lh * artK * ASPECT); }
  tctx.setTransform(1, 0, 0, 1, 0, 0);
  tctx.clearRect(0, 0, lw, lh);
  drawTexts(tctx, texts);
  drawCursor(tctx);
}

// ------------------------------------------------------------------ Pagan's direction cursor
// With a mouse, the pointer is an arrow pointing away from the avatar. Like
// Pagan's, it comes in three lengths: short (careful step), medium (walk) and
// long (run), set by how far the pointer is from the avatar.
const fineMouse = window.matchMedia && matchMedia('(pointer: fine)').matches;
let mouseAt = null;
if (fineMouse) document.body.classList.add('arrowcursor');
function drawCursor(c) {
  if (!fineMouse || !mouseAt || !ex || !ex.avatar_sx || !$('create').classList.contains('hidden')) return;
  const ax = ex.avatar_sx(), ay = ex.avatar_sy();
  const mx = mouseAt[0], my = mouseAt[1];
  const dx = mx - ax, dy = my - ay;
  const d = Math.hypot(dx, dy);
  // eight directions, as the avatar can face
  const a = Math.round(Math.atan2(dy, dx) / (Math.PI / 4)) * (Math.PI / 4);
  const len = d < 50 ? 5 : d < 140 ? 10 : 16;
  const X = Math.round(mx / pix), Y = Math.round(my / pix);
  const ux = Math.cos(a), uy = Math.sin(a), px = -uy, py = ux;
  const tail = [X - ux * len, Y - uy * len];
  const pts = [
    [X, Y],
    [X - ux * 5 + px * 3.5, Y - uy * 5 + py * 3.5],
    [X - ux * 5 + px * 1.2, Y - uy * 5 + py * 1.2],
    [tail[0] + px * 1.2, tail[1] + py * 1.2],
    [tail[0] - px * 1.2, tail[1] - py * 1.2],
    [X - ux * 5 - px * 1.2, Y - uy * 5 - py * 1.2],
    [X - ux * 5 - px * 3.5, Y - uy * 5 - py * 3.5],
  ];
  c.setTransform(1, 0, 0, 1, 0, 0);
  c.beginPath();
  pts.forEach(([x, y], i) => (i ? c.lineTo(x, y) : c.moveTo(x, y)));
  c.closePath();
  c.fillStyle = '#c8481c';
  c.fill();
  c.lineWidth = 1;
  c.strokeStyle = '#120c08';
  c.stroke();
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
  // shown at exactly the size it is drawn for: 100vw/100vh can differ from
  // innerWidth/innerHeight (Safari's toolbars), and the browser would then
  // resample the whole picture
  canvas.style.width = `${canvas.width / dpr}px`;
  canvas.style.height = `${canvas.height / dpr}px`;
  artK = Math.max(1, Math.round(canvas.width / 320));
  pix = artK / dpr;
  if (ex) { ex.resize(w, h / ASPECT); ex.set_pix(pix); }
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
  isGhost = !!h.ghost;
  $('pname').textContent = h.ghost ? `${h.name} (ghost)` : h.name;
  // Pagan's status lights: red for hit points, blue for mana, shrinking toward zero
  $('hpb').style.height = `${(100 * h.hp / Math.max(1, h.mhp)).toFixed(1)}%`;
  $('hpt').textContent = `${h.hp}/${h.mhp}`;
  $('mpb').style.height = `${(100 * h.mana / Math.max(1, h.mmana)).toFixed(1)}%`;
  $('mpt').textContent = `${h.mana}/${h.mmana}`;
  $('period').textContent = h.period || '';
  if (font) { const g = $('gold'); g.textContent = ''; g.appendChild(icon(10)); g.append(` ${h.gold}`); }
  else $('gold').textContent = `🪙 ${h.gold}`;
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
let isGhost = false;
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
  // conversations float over the world like Pagan's: no frame, replies as red keywords
  win.classList.toggle('talk', w.id === 'ctx');
  // the living hear the line spoken over the speaker's head; a ghost reads it here
  win.classList.toggle('spoken', w.id === 'ctx' && !isGhost);
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
  if (w.head.length || w.doll) {
    const hd = el('div', 'head');
    // the paperdoll: the hero as the game baked them, in their gear
    if (w.doll && atlases.get(w.doll[0])) {
      const [id, f, fw, fh] = w.doll;
      const src = atlases.get(id);
      const cv = document.createElement('canvas');
      cv.width = fw; cv.height = fh;
      cv.getContext('2d').drawImage(src, (f % 16) * fw, Math.floor(f / 16) * fh, fw, fh, 0, 0, fw, fh);
      cv.className = 'doll';
      cv.style.width = `${fw * 3}px`;
      cv.style.height = `${fh * 3 * ASPECT}px`;
      hd.classList.add('withdoll');
      hd.appendChild(cv);
    }
    const lines = el('div', 'lines');
    for (const line of w.head) lines.appendChild(el('div', '', line));
    hd.appendChild(lines);
    win.appendChild(hd);
  }
  if (w.rows.length) {
    const rows = el('div', 'rows');
    for (const r of w.rows) {
      const row = el('div', 'r' + (r.dim ? ' dim' : ''));
      if (font && r.i.startsWith('#')) { const d = el('div', 'ic'); d.appendChild(icon(+r.i.slice(1))); row.appendChild(d); }
      else row.appendChild(el('div', 'ic', r.i));
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
function rel(e) { return [e.clientX, e.clientY / ASPECT]; }
canvas.addEventListener('pointerdown', (e) => {
  canvas.setPointerCapture(e.pointerId);
  pointers.set(e.pointerId, rel(e));
  if (pointers.size === 1) ex.ptr_down(...rel(e));
  else { ex.ptr_cancel(); pinchD = pinchDist(); }
});
canvas.addEventListener('pointerleave', () => { mouseAt = null; });
canvas.addEventListener('pointermove', (e) => {
  if (e.pointerType === 'mouse') mouseAt = rel(e);
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
  else if (k === 'KeyE') cmd('ride');
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
  const imports = { env: { js_log: (p, l) => console.error(readStr(p, l)), js_now: () => performance.now() } };
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
  if (ex.ui_img_w) initUiArt();
  resize();
  buildMinimap();
  $('loading').classList.add('hidden');
  refreshUI(true);
  requestAnimationFrame((t) => { last = t; loop(t); });
}
boot().catch((e) => { $('loading').textContent = 'Failed to load the realm: ' + e; console.error(e); });
