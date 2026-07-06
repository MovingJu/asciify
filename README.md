# asciify

이미지를 올리면 ASCII 아트로, GIF를 올리면 프레임별로 변환해서 애니메이션 그대로 재생하는
웹 앱. 변환 로직은 전부 Rust로 짜서 WebAssembly로 컴파일하고, 브라우저에서 직접 실행한다
(서버로 이미지를 업로드해서 처리하는 방식이 아니라 100% 클라이언트 사이드).

## 아키텍처

```
asciify/
  core/              ← Rust. 이미지/GIF 디코딩 + ASCII 변환 로직. wasm-pack으로 빌드
    src/lib.rs
    Cargo.toml
  web/               ← 프론트엔드. 업로드 UI + wasm 호출 + 애니메이션 재생
    index.html
    main.js
    serve.ts         ← Bun 정적 서버
    pkg/             ← wasm-pack 빌드 결과물 (git에는 안 올림, 매번 빌드)
```

```
┌──────────────┐   파일 업로드    ┌──────────────────┐
│   브라우저    │ ───────────────> │  core (WASM)      │
│  (main.js)   │                  │  image_to_ascii    │
│              │ <─────────────── │  gif_to_ascii_frames │
└──────────────┘   ASCII 문자열    └──────────────────┘
```

## 빌드 & 실행

```bash
# 1. Rust → WASM 빌드
cd core
wasm-pack build --target web --release --out-dir ../web/pkg --out-name asciify_core

# 2. 웹 서버 실행
cd ../web
bun run serve.ts
# http://localhost:8080 접속
```

## 핵심 아이디어 — 이미지를 어떻게 ASCII로 바꾸나

1. 이미지를 원하는 가로 문자 수(`cols`)에 맞게 축소한다.
   - **터미널 글자는 정사각형이 아니라 세로로 길다** (대략 가로:세로 = 1:2).
     그래서 세로 문자 수는 `cols * (원본 세로/가로 비율) * 0.5` 정도로 보정해야
     결과물이 찌그러지지 않는다. 이걸 안 하면 ASCII 아트가 세로로 두 배 늘어난 것처럼 보인다.
2. 각 픽셀을 흑백(밝기)으로 변환한다.
3. 밝기(0~255)를 문자 램프(` .:-=+*#%@`, 밝은 순서→어두운 순서)의 인덱스로 매핑한다.
4. 한 줄씩 이어붙여서 완성.

GIF는 이 과정을 프레임마다 반복하고, 각 프레임의 재생 시간(delay)까지 같이 넘겨서
프론트엔드가 `setTimeout`으로 애니메이션처럼 재생한다.

## 진행 상황

지금은 **뼈대만 완성**된 상태다:
- ✅ Rust → WASM 빌드 파이프라인 (wasm-pack, `cdylib` 설정, `image` 크레이트 wasm32 호환 확인됨)
- ✅ 프론트엔드 UI (드래그앤드롭 업로드, 슬라이더로 해상도 조절, 애니메이션 재생 로직)
- ⬜ **실제 변환 로직** (`core/src/lib.rs`의 `image_to_ascii`, `gif_to_ascii_frames`) — 지금은 `todo!()`

실제 알고리즘 구현은 [Issues](../../issues)에 단계별로 나눠뒀다. `good-first-issue` 라벨부터 시작하면 된다.

## 참고

- [`image` 크레이트 문서](https://docs.rs/image/latest/image/)
- [ASCII art 밝기 매핑 알고리즘 (Paul Bourke)](https://paulbourke.net/dataformats/asciiart/)
- 관련 문서: [SIMD 프로그래밍](https://berry.movingju.com/rust-book/performance/simd.html) — 픽셀 처리량이 많아지면 SIMD로 가속하는 것도 고려해볼 만하다 (이슈 #9 참고)
