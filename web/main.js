import init, { image_to_ascii, gif_to_ascii_frames } from "./pkg/asciify_core.js";
import { encodeGif } from "./gif-encoder.js";

let wasmReady = null;
function ensureWasm() {
  if (!wasmReady) wasmReady = init();
  return wasmReady;
}

const dropZone = document.getElementById("drop-zone");
const fileInput = document.getElementById("file-input");
const colsSlider = document.getElementById("cols-slider");
const colsValue = document.getElementById("cols-value");
const output = document.getElementById("output");
const status = document.getElementById("status");
const downloadTextBtn = document.getElementById("download-text-btn");
const downloadImageBtn = document.getElementById("download-image-btn");
const downloadGifBtn = document.getElementById("download-gif-btn");

let currentAnimation = null; // 재생 중인 GIF 애니메이션의 setTimeout 핸들
let lastAscii = null; // 지금 화면에 보이는 ASCII 결과 (다운로드 대상, GIF는 현재 프레임 기준)
let lastFrames = null; // 지금 결과가 GIF일 때 전체 프레임 배열 [{ascii, delayMs}, ...], 아니면 null

// Rust 쪽이 아직 크기 제한을 검사하지 않아서(이슈 #19), 너무 큰 이미지를 wasm에 넘기면
// 리사이즈/밝기 계산 중에 탭이 멈출 수 있다. 최종 해결은 이슈 #19에서 다루고,
// 여기선 임시로 프론트에서 먼저 걸러낸다.
const MAX_PIXELS = 20_000_000; // ~5000x4000, Rust 쪽 프로토타입(PR #18)에서 쓴 것과 같은 한도

colsSlider.addEventListener("input", () => {
  colsValue.textContent = colsSlider.value;
});

dropZone.addEventListener("click", () => fileInput.click());

dropZone.addEventListener("dragover", (e) => {
  e.preventDefault();
  dropZone.classList.add("drag-over");
});
dropZone.addEventListener("dragleave", () => dropZone.classList.remove("drag-over"));
dropZone.addEventListener("drop", (e) => {
  e.preventDefault();
  dropZone.classList.remove("drag-over");
  const file = e.dataTransfer.files[0];
  if (file) handleFile(file);
});

fileInput.addEventListener("change", () => {
  const file = fileInput.files[0];
  if (file) handleFile(file);
});

async function handleFile(file) {
  if (currentAnimation) {
    clearTimeout(currentAnimation);
    currentAnimation = null;
  }

  status.textContent = "변환 중...";
  output.textContent = "";
  setDownloadResult(null);
  lastFrames = null;
  downloadGifBtn.disabled = true;

  try {
    const bitmap = await createImageBitmap(file);
    const { width, height } = bitmap;
    bitmap.close(); // close() 이후엔 width/height가 0이 되므로 미리 꺼내둠

    if (width * height > MAX_PIXELS) {
      status.textContent =
        `이미지가 너무 큽니다 (${width}x${height}). ` +
        `최대 ${MAX_PIXELS.toLocaleString()}픽셀까지 지원하니 더 작은 이미지로 시도해주세요.`;
      return;
    }

    await ensureWasm();
    const bytes = new Uint8Array(await file.arrayBuffer());
    const cols = Number(colsSlider.value);

    if (file.type === "image/gif") {
      const framesJson = gif_to_ascii_frames(bytes, cols);
      const frames = JSON.parse(framesJson); // [{ ascii, delayMs }, ...]
      lastFrames = frames;
      downloadGifBtn.disabled = false;
      playAnimation(frames);
      status.textContent = `${frames.length}프레임 GIF 재생 중`;
    } else {
      lastFrames = null;
      downloadGifBtn.disabled = true;
      const ascii = image_to_ascii(bytes, cols);
      output.textContent = ascii;
      setDownloadResult(ascii);
      status.textContent = "완료";
    }
  } catch (err) {
    status.textContent = `에러: ${err.message ?? err}`;
    console.error(err);
  }
}

function playAnimation(frames) {
  let i = 0;
  function tick() {
    output.textContent = frames[i].ascii;
    setDownloadResult(frames[i].ascii); // 다운로드는 항상 "지금 보이는 프레임" 기준
    i = (i + 1) % frames.length;
    currentAnimation = setTimeout(tick, frames[i === 0 ? frames.length - 1 : i - 1].delayMs || 100);
  }
  tick();
}

// 다운로드 버튼이 참조할 현재 결과를 갱신하고, 있고 없음에 따라 버튼 활성/비활성 처리.
function setDownloadResult(ascii) {
  lastAscii = ascii;
  downloadTextBtn.disabled = !ascii;
  downloadImageBtn.disabled = !ascii;
}

downloadTextBtn.addEventListener("click", () => {
  if (!lastAscii) return;
  const blob = new Blob([lastAscii], { type: "text/plain" });
  triggerDownload(URL.createObjectURL(blob), "ascii-art.txt");
});

downloadImageBtn.addEventListener("click", () => {
  if (!lastAscii) return;
  const canvas = renderAsciiToCanvas(lastAscii);
  canvas.toBlob((blob) => {
    triggerDownload(URL.createObjectURL(blob), "ascii-art.png");
  }, "image/png");
});

downloadGifBtn.addEventListener("click", () => {
  if (!lastFrames) return;
  // 모든 프레임을 같은 캔버스 크기로 그려야 하므로, 첫 프레임 크기를 기준으로 맞춘다
  // (같은 GIF에서 나온 프레임들은 cols가 고정이라 사실상 항상 크기가 같음).
  const rendered = lastFrames.map(({ ascii, delayMs }) => {
    const canvas = renderAsciiToCanvas(ascii);
    const ctx = canvas.getContext("2d");
    return { imageData: ctx.getImageData(0, 0, canvas.width, canvas.height), delayMs };
  });

  const gifBytes = encodeGif(rendered);
  const blob = new Blob([gifBytes], { type: "image/gif" });
  triggerDownload(URL.createObjectURL(blob), "ascii-art.gif");
});

// ASCII 문자열을 모노스페이스 폰트로 캔버스에 그려서 리턴한다 (PNG 내보내기용).
function renderAsciiToCanvas(ascii) {
  const fontSize = 12;
  const lineHeight = fontSize; // #output의 line-height:1.0과 맞춤
  const font = `${fontSize}px monospace`;

  const lines = ascii.split("\n");
  const maxCols = Math.max(1, ...lines.map((line) => line.length));

  // 글자 폭은 실제로 측정해야 캔버스 크기가 정확히 맞는다 (모노스페이스라 폭이 모두 동일).
  const measureCtx = document.createElement("canvas").getContext("2d");
  measureCtx.font = font;
  const charWidth = measureCtx.measureText("M").width;

  const canvas = document.createElement("canvas");
  canvas.width = Math.ceil(charWidth * maxCols);
  canvas.height = Math.ceil(lineHeight * lines.length);

  const ctx = canvas.getContext("2d");
  ctx.fillStyle = "#111"; // #output 배경색과 맞춤
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  ctx.fillStyle = "#eee"; // #output 글자색과 맞춤
  ctx.font = font;
  ctx.textBaseline = "top";
  lines.forEach((line, i) => ctx.fillText(line, 0, i * lineHeight));

  return canvas;
}

function triggerDownload(url, filename) {
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}
