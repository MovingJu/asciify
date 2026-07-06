import init, { image_to_ascii, gif_to_ascii_frames } from "./pkg/asciify_core.js";

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

let currentAnimation = null; // 재생 중인 GIF 애니메이션의 setTimeout 핸들

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

  try {
    await ensureWasm();
    const bytes = new Uint8Array(await file.arrayBuffer());
    const cols = Number(colsSlider.value);

    if (file.type === "image/gif") {
      const framesJson = gif_to_ascii_frames(bytes, cols);
      const frames = JSON.parse(framesJson); // [{ ascii, delayMs }, ...]
      playAnimation(frames);
      status.textContent = `${frames.length}프레임 GIF 재생 중`;
    } else {
      const ascii = image_to_ascii(bytes, cols);
      output.textContent = ascii;
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
    i = (i + 1) % frames.length;
    currentAnimation = setTimeout(tick, frames[i === 0 ? frames.length - 1 : i - 1].delayMs || 100);
  }
  tick();
}
