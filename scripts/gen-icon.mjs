// 生成一张 1024x1024 的启动台风格图标源图（深色渐变 + 3x3 圆角白块），
// 供 `npx tauri icon app-icon.png` 生成各平台/尺寸图标。纯 Node，无外部依赖。
import zlib from "node:zlib";
import { writeFileSync } from "node:fs";

const W = 1024;
const H = 1024;

// 每行首字节为 PNG filter(0=None)；行内 RGBA。
const rowLen = W * 4 + 1;
const raw = Buffer.alloc(rowLen * H);

function setPx(x, y, r, g, b, a) {
  if (x < 0 || x >= W || y < 0 || y >= H) return;
  const i = y * rowLen + 1 + x * 4;
  raw[i] = r;
  raw[i + 1] = g;
  raw[i + 2] = b;
  raw[i + 3] = a;
}

function roundedRect(x0, y0, x1, y1, r, [R, G, B, A]) {
  for (let y = Math.floor(y0); y < Math.ceil(y1); y++) {
    for (let x = Math.floor(x0); x < Math.ceil(x1); x++) {
      if (x < 0 || y < 0 || x >= W || y >= H) continue;
      const cx = Math.min(Math.max(x, x0 + r), x1 - 1 - r);
      const cy = Math.min(Math.max(y, y0 + r), y1 - 1 - r);
      const dx = x - cx;
      const dy = y - cy;
      const insideCorner = dx * dx + dy * dy <= r * r;
      const insideEdge = (x >= x0 + r && x < x1 - r) || (y >= y0 + r && y < y1 - r);
      if (insideCorner || insideEdge) setPx(x, y, R, G, B, A);
    }
  }
}

// 背景：深蓝紫渐变
for (let y = 0; y < H; y++) {
  const t = y / H;
  const r = Math.round(18 + 28 * t);
  const g = Math.round(26 + 18 * t);
  const b = Math.round(74 + 60 * t);
  for (let x = 0; x < W; x++) setPx(x, y, r, g, b, 255);
}

// 3x3 白色圆角方块（启动台网格意象）
const cells = 3;
const pad = 150;
const gap = 70;
const cellSize = (W - pad * 2 - gap * (cells - 1)) / cells;
const radius = Math.round(cellSize * 0.28);
for (let cy = 0; cy < cells; cy++) {
  for (let cx = 0; cx < cells; cx++) {
    const x0 = pad + cx * (cellSize + gap);
    const y0 = pad + cy * (cellSize + gap);
    roundedRect(x0, y0, x0 + cellSize, y0 + cellSize, radius, [255, 255, 255, 255]);
  }
}

// --- PNG 编码 ---
const crcTable = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();
function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = crcTable[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
  const typeBuf = Buffer.from(type, "ascii");
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])), 0);
  return Buffer.concat([len, typeBuf, data, crc]);
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(W, 0);
ihdr.writeUInt32BE(H, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // color type RGBA
ihdr[10] = 0;
ihdr[11] = 0;
ihdr[12] = 0;

const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
const png = Buffer.concat([
  sig,
  chunk("IHDR", ihdr),
  chunk("IDAT", zlib.deflateSync(raw)),
  chunk("IEND", Buffer.alloc(0)),
]);

writeFileSync("app-icon.png", png);
console.log(`生成 app-icon.png (${W}x${H}, ${png.length} bytes)`);
