// Generates a placeholder 1024x1024 source icon (solid deep-blue square).
// Replace assets/icon-source.png with the real brand artwork, then re-run:
//   pnpm --filter @dsh-desktop/desktop tauri icon ../../assets/icon-source.png
import { deflateSync } from "node:zlib";
import { writeFileSync, mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const W = 1024;
const H = 1024;

function crc32(buf) {
  let c = ~0;
  for (let i = 0; i < buf.length; i++) {
    c ^= buf[i];
    for (let k = 0; k < 8; k++) c = (c >>> 1) ^ (0xedb88320 & -(c & 1));
  }
  return ~c >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const typeBuf = Buffer.from(type, "ascii");
  const crcBuf = Buffer.alloc(4);
  crcBuf.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])));
  return Buffer.concat([len, typeBuf, data, crcBuf]);
}

const raw = Buffer.alloc(W * H * 4);
for (let y = 0; y < H; y++) {
  for (let x = 0; x < W; x++) {
    const i = (y * W + x) * 4;
    raw[i] = 0x0d;
    raw[i + 1] = 0x17;
    raw[i + 2] = 0x28;
    raw[i + 3] = 0xff;
  }
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(W, 0);
ihdr.writeUInt32BE(H, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // color type RGBA
ihdr[10] = 0;
ihdr[11] = 0;
ihdr[12] = 0;

const filtered = Buffer.alloc(H * (W * 4 + 1));
for (let y = 0; y < H; y++) {
  filtered[y * (W * 4 + 1)] = 0;
  raw.copy(filtered, y * (W * 4 + 1) + 1, y * W * 4, (y + 1) * W * 4);
}

const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", ihdr),
  chunk("IDAT", deflateSync(filtered)),
  chunk("IEND", Buffer.alloc(0)),
]);

const here = dirname(fileURLToPath(import.meta.url));
const out = join(here, "..", "assets", "icon-source.png");
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, png);
console.log(`wrote ${out} (${png.length} bytes)`);
