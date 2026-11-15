// PNG에서 크기와 알파(가로×세로 바이트)를 꺼낸다. 화면은 캔버스로 읽지만 Node에는 캔버스가 없어 직접 푼다.
// 인터레이스 PNG나 드문 형식은 알파 없이 크기만 돌려준다(메시 자동 생성만 못 쓰고 나머지는 된다).
import { inflateSync } from "node:zlib";

const SIGNATURE = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
/** 색 종류 → 픽셀당 채널 수 */
const CHANNELS = { 0: 1, 2: 3, 3: 1, 4: 2, 6: 4 };

function paeth(a, b, c) {
  const p = a + b - c;
  const pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c);
  return pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
}

/** 줄마다 붙은 필터를 풀어 순수 픽셀 바이트로 만든다 */
function unfilter(raw, rowBytes, bpp, height) {
  const out = Buffer.alloc(rowBytes * height);
  for (let y = 0; y < height; y++) {
    const type = raw[y * (rowBytes + 1)];
    const src = y * (rowBytes + 1) + 1, dst = y * rowBytes, up = dst - rowBytes;
    for (let i = 0; i < rowBytes; i++) {
      const a = i >= bpp ? out[dst + i - bpp] : 0;
      const b = y > 0 ? out[up + i] : 0;
      const c = y > 0 && i >= bpp ? out[up + i - bpp] : 0;
      const pred = type === 1 ? a : type === 2 ? b : type === 3 ? (a + b) >> 1 : type === 4 ? paeth(a, b, c) : 0;
      out[dst + i] = (raw[src + i] + pred) & 0xff;
    }
  }
  return out;
}

/** @returns {{width: number, height: number, alpha?: Uint8Array}} */
export function readPng(bytes) {
  if (bytes.length < 33 || SIGNATURE.some((b, i) => bytes[i] !== b)) throw new Error("PNG 파일이 아닙니다");
  const buf = Buffer.from(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let width = 0, height = 0, depth = 0, color = 0, interlace = 0;
  let trns = null;
  const idat = [];
  for (let p = 8; p + 12 <= buf.length; ) {
    const len = buf.readUInt32BE(p);
    const type = buf.toString("ascii", p + 4, p + 8);
    const data = buf.subarray(p + 8, p + 8 + len);
    if (type === "IHDR") {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      depth = data[8];
      color = data[9];
      interlace = data[12];
    } else if (type === "tRNS") trns = data;
    else if (type === "IDAT") idat.push(data);
    else if (type === "IEND") break;
    p += 12 + len;
  }
  if (!width || !height) throw new Error("PNG 크기를 읽지 못했습니다");
  const size = { width, height };
  const channels = CHANNELS[color];
  if (interlace || !channels || !idat.length) return size;

  const opaque = () => ({ ...size, alpha: new Uint8Array(width * height).fill(255) });
  // 알파 채널도 팔레트 투명도도 없으면 전부 불투명
  if (color === 0 || color === 2 || (color === 3 && !trns)) return opaque();
  // 알파 채널은 8·16비트만 있다. 팔레트는 8비트 이하
  if (color !== 3 && depth !== 8 && depth !== 16) return size;

  const bitsPerPixel = channels * depth;
  const rowBytes = Math.ceil((width * bitsPerPixel) / 8);
  let pixels;
  try {
    const raw = inflateSync(Buffer.concat(idat));
    if (raw.length < (rowBytes + 1) * height) return size;
    pixels = unfilter(raw, rowBytes, Math.max(1, bitsPerPixel >> 3), height);
  } catch {
    return size;
  }

  const alpha = new Uint8Array(width * height);
  if (color === 3) {
    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        const bit = x * depth;
        const index = (pixels[y * rowBytes + (bit >> 3)] >> (8 - depth - (bit & 7))) & ((1 << depth) - 1);
        alpha[y * width + x] = index < trns.length ? trns[index] : 255;
      }
    }
  } else {
    const step = (channels * depth) / 8;
    // 알파는 마지막 채널. 16비트면 윗바이트를 쓴다
    const offset = (channels - 1) * (depth / 8);
    for (let i = 0; i < alpha.length; i++) alpha[i] = pixels[i * step + offset];
  }
  return { ...size, alpha };
}
