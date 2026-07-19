// 인코더 단독 검증용 스크립트 (node로 직접 실행, 프로젝트 런타임의 일부가 아님).
// 사용: node web/gif-encoder.selftest.mjs
import { encodeGif } from "./gif-encoder.js";
import { writeFileSync } from "node:fs";

function fakeImageData(width, height, rgb) {
  const data = new Uint8ClampedArray(width * height * 4);
  for (let p = 0; p < width * height; p++) {
    data[p * 4] = rgb[0];
    data[p * 4 + 1] = rgb[1];
    data[p * 4 + 2] = rgb[2];
    data[p * 4 + 3] = 255;
  }
  return { data, width, height };
}

const width = 4;
const height = 4;
const frames = [
  { imageData: fakeImageData(width, height, [17, 17, 17]), delayMs: 100 }, // #111
  { imageData: fakeImageData(width, height, [238, 238, 238]), delayMs: 200 }, // #eee
  { imageData: fakeImageData(width, height, [255, 0, 0]), delayMs: 150 },
];

const bytes = encodeGif(frames);

function assert(cond, msg) {
  if (!cond) throw new Error("FAIL: " + msg);
  console.log("ok:", msg);
}

const header = String.fromCharCode(...bytes.slice(0, 6));
assert(header === "GIF89a", "헤더가 GIF89a");
assert(bytes[bytes.length - 1] === 0x3b, "트레일러가 0x3B");

// NETSCAPE2.0 루프 익스텐션이 있는지
const netscapeBytes = [0x4e, 0x45, 0x54, 0x53, 0x43, 0x41, 0x50, 0x45, 0x32, 0x2e, 0x30]; // "NETSCAPE2.0"
let found = false;
for (let i = 0; i < bytes.length - netscapeBytes.length; i++) {
  if (netscapeBytes.every((b, j) => bytes[i + j] === b)) {
    found = true;
    break;
  }
}
assert(found, "NETSCAPE2.0 루프 익스텐션 포함");

// Graphic Control Extension(0x21 0xF9) 개수 == 프레임 수
let gceCount = 0;
let idCount = 0; // Image Descriptor(0x2C) 개수
for (let i = 0; i < bytes.length - 1; i++) {
  if (bytes[i] === 0x21 && bytes[i + 1] === 0xf9) gceCount++;
  if (bytes[i] === 0x2c) idCount++;
}
assert(gceCount === frames.length, `GCE 개수 == ${frames.length} (실제 ${gceCount})`);
assert(idCount === frames.length, `Image Descriptor 개수 == ${frames.length} (실제 ${idCount})`);

// width/height가 Logical Screen Descriptor에 LE로 들어갔는지 (offset 6,8)
const w = bytes[6] | (bytes[7] << 8);
const h = bytes[8] | (bytes[9] << 8);
assert(w === width && h === height, `크기가 ${width}x${height} (실제 ${w}x${h})`);

writeFileSync("/tmp/selftest.gif", Buffer.from(bytes));
console.log("wrote /tmp/selftest.gif,", bytes.length, "bytes");
console.log("ALL STRUCTURAL CHECKS PASSED");
