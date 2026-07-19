// 의존성 없는 최소 GIF89a 인코더.
// main.js에서 각 프레임을 canvas에 그려서 뽑은 ImageData 배열을 실제 애니메이션 GIF 바이트로 만든다.
// (프로젝트에 번들러가 없어서 서드파티 GIF 라이브러리를 검증 없이 그대로 들여오는 대신 직접 작성함 —
//  모든 프레임이 불투명 전체 리드로우라서 부분 프레임 합성/투명 처리는 필요 없는, 스펙상 딱 필요한 만큼만 구현)

/**
 * frames: [{ imageData: {data: Uint8ClampedArray, width, height}, delayMs: number }, ...]
 * 모든 프레임은 같은 width/height여야 한다 (첫 프레임 기준).
 * 반환: Uint8Array (GIF89a 바이트)
 */
export function encodeGif(frames) {
  if (!frames || frames.length === 0) {
    throw new Error("encodeGif: frames가 비어있음");
  }
  const width = frames[0].imageData.width;
  const height = frames[0].imageData.height;

  const { palette, indexedFrames } = buildGlobalPalette(frames, width, height);
  const bytes = new ByteWriter();

  writeHeader(bytes, width, height, palette);
  writeLoopExtension(bytes);
  for (let i = 0; i < indexedFrames.length; i++) {
    writeFrame(bytes, width, height, palette.length, indexedFrames[i], frames[i].delayMs);
  }
  bytes.push(0x3b); // trailer

  return bytes.toUint8Array();
}

// ---- 팔레트 ----

// 모든 프레임의 색을 모아서 팔레트(최대 256색)를 만들고, 각 프레임을 팔레트 인덱스 배열로 바꾼다.
function buildGlobalPalette(frames, width, height) {
  const pixelCount = width * height;

  // 우선 원본 정밀도(24bit RGB) 그대로 유니크 색을 모아본다.
  let colorSet = collectColors(frames, (r, g, b) => (r << 16) | (g << 8) | b);

  // 256색을 넘으면 채널별 비트를 줄여서(더 거친 양자화) 다시 모은다.
  // 이 프로젝트의 실제 입력(배경 #111 + 텍스트 #eee + 안티에일리어싱으로 생기는 중간 회색조)은
  // 256색을 넘을 일이 거의 없지만, 안전장치로 둔다.
  let shift = 0;
  while (colorSet.size > 256 && shift < 8) {
    shift++;
    colorSet = collectColors(frames, quantizeKey(shift));
  }
  if (colorSet.size > 256) {
    throw new Error("encodeGif: 색상을 256개 이하로 줄이지 못함");
  }

  const keyFn = shift === 0 ? (r, g, b) => (r << 16) | (g << 8) | b : quantizeKey(shift);

  const keys = Array.from(colorSet.keys());
  const keyToIndex = new Map();
  keys.forEach((key, idx) => keyToIndex.set(key, idx));

  // 팔레트 크기는 2의 거듭제곱(최소 2)이어야 함 — 남는 칸은 검정으로 채운다.
  let paletteSize = 2;
  while (paletteSize < keys.length) paletteSize *= 2;

  const palette = new Array(paletteSize);
  for (let i = 0; i < paletteSize; i++) {
    if (i < keys.length) {
      const key = keys[i];
      palette[i] = [(key >> 16) & 0xff, (key >> 8) & 0xff, key & 0xff];
    } else {
      palette[i] = [0, 0, 0];
    }
  }

  const indexedFrames = frames.map(({ imageData }) => {
    const { data } = imageData;
    const indices = new Uint8Array(pixelCount);
    for (let p = 0; p < pixelCount; p++) {
      const o = p * 4;
      const key = keyFn(data[o], data[o + 1], data[o + 2]);
      indices[p] = keyToIndex.get(key);
    }
    return indices;
  });

  return { palette, indexedFrames };
}

function quantizeKey(shift) {
  return (r, g, b) => ((r >> shift) << 16) | ((g >> shift) << 8) | (b >> shift);
}

function collectColors(frames, keyFn) {
  const set = new Set();
  for (const { imageData } of frames) {
    const { data, width, height } = imageData;
    const pixelCount = width * height;
    for (let p = 0; p < pixelCount; p++) {
      const o = p * 4;
      set.add(keyFn(data[o], data[o + 1], data[o + 2]));
    }
  }
  return set;
}

// ---- GIF 구조 ----

function writeHeader(bytes, width, height, palette) {
  bytes.pushString("GIF89a");

  bytes.pushU16(width);
  bytes.pushU16(height);

  const colorTableSizeBits = Math.log2(palette.length) - 1; // palette.length = 2^(n+1)
  const packed =
    0x80 | // Global Color Table Flag = 1
    (0x07 << 4) | // Color Resolution = 111 (사실상 무의미, 관례적으로 최대값 사용)
    (0x00 << 3) | // Sort Flag = 0
    colorTableSizeBits;
  bytes.push(packed);

  bytes.push(0x00); // Background Color Index
  bytes.push(0x00); // Pixel Aspect Ratio

  for (const [r, g, b] of palette) {
    bytes.push(r, g, b);
  }
}

function writeLoopExtension(bytes) {
  bytes.push(0x21, 0xff, 0x0b);
  bytes.pushString("NETSCAPE2.0");
  bytes.push(0x03, 0x01);
  bytes.pushU16(0x0000); // loop count 0 = 무한 반복
  bytes.push(0x00);
}

function writeFrame(bytes, width, height, paletteLength, indices, delayMs) {
  // Graphic Control Extension
  bytes.push(0x21, 0xf9, 0x04);
  bytes.push(0x04); // disposal method = 1 (do not dispose) << 2, 투명/유저입력 플래그 없음
  bytes.pushU16(Math.round((delayMs || 100) / 10)); // centiseconds
  bytes.push(0x00); // transparent color index (미사용)
  bytes.push(0x00);

  // Image Descriptor
  bytes.push(0x2c);
  bytes.pushU16(0); // left
  bytes.pushU16(0); // top
  bytes.pushU16(width);
  bytes.pushU16(height);
  bytes.push(0x00); // local color table 없음, interlace 없음

  const minCodeSize = Math.max(2, Math.ceil(Math.log2(paletteLength)));
  bytes.push(minCodeSize);

  const compressed = lzwEncode(indices, minCodeSize);
  writeSubBlocks(bytes, compressed);
}

function writeSubBlocks(bytes, data) {
  let offset = 0;
  while (offset < data.length) {
    const chunkSize = Math.min(255, data.length - offset);
    bytes.push(chunkSize);
    for (let i = 0; i < chunkSize; i++) bytes.push(data[offset + i]);
    offset += chunkSize;
  }
  bytes.push(0x00); // block terminator
}

// GIF 방식 LZW: 코드를 LSB-first로 비트에 채워넣는다 (여기서 순서가 틀리면 전부 깨진 그림이 됨).
function lzwEncode(indices, minCodeSize) {
  const clearCode = 1 << minCodeSize;
  const endCode = clearCode + 1;
  let codeSize = minCodeSize + 1;
  const maxCodeValue = 4096;

  const out = new BitWriter();

  let dict, nextCode;
  function resetDict() {
    dict = new Map();
    for (let i = 0; i < clearCode; i++) dict.set(String(i), i);
    nextCode = endCode + 1;
    codeSize = minCodeSize + 1;
  }

  resetDict();
  out.writeCode(clearCode, codeSize);

  let prefix = "";
  for (let i = 0; i < indices.length; i++) {
    const k = indices[i];
    const combined = prefix === "" ? String(k) : prefix + "," + k;
    if (dict.has(combined)) {
      prefix = combined;
    } else {
      out.writeCode(dict.get(prefix), codeSize);
      if (nextCode < maxCodeValue) {
        dict.set(combined, nextCode);
        nextCode++;
        if (nextCode > 1 << codeSize && codeSize < 12) {
          codeSize++;
        }
      } else {
        // 사전이 꽉 찼으니 Clear Code로 리셋
        out.writeCode(clearCode, codeSize);
        resetDict();
      }
      prefix = String(k);
    }
  }
  if (prefix !== "") {
    out.writeCode(dict.get(prefix), codeSize);
  }
  out.writeCode(endCode, codeSize);

  return out.toUint8Array();
}

// ---- 바이트/비트 유틸 ----

class ByteWriter {
  constructor() {
    this.bytes = [];
  }
  push(...vals) {
    for (const v of vals) this.bytes.push(v & 0xff);
  }
  pushU16(v) {
    this.bytes.push(v & 0xff, (v >> 8) & 0xff);
  }
  pushString(s) {
    for (let i = 0; i < s.length; i++) this.bytes.push(s.charCodeAt(i));
  }
  toUint8Array() {
    return new Uint8Array(this.bytes);
  }
}

// LSB-first 비트 패킹 (GIF LZW 스펙 요구사항).
class BitWriter {
  constructor() {
    this.bytes = [];
    this.bitBuffer = 0;
    this.bitCount = 0;
  }
  writeCode(code, codeSize) {
    this.bitBuffer |= code << this.bitCount;
    this.bitCount += codeSize;
    while (this.bitCount >= 8) {
      this.bytes.push(this.bitBuffer & 0xff);
      this.bitBuffer >>= 8;
      this.bitCount -= 8;
    }
  }
  toUint8Array() {
    if (this.bitCount > 0) {
      this.bytes.push(this.bitBuffer & 0xff);
      this.bitBuffer = 0;
      this.bitCount = 0;
    }
    return new Uint8Array(this.bytes);
  }
}
