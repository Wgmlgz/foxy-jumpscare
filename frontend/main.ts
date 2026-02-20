import { getCurrentWindow } from '@tauri-apps/api/window';
import { decompressFrames, parseGIF, type ParsedFrame } from 'gifuct-js';

const GIF_SRC = './media/foxy.gif';
const AUDIO_SRC = './media/scream.mp3';
const FAILSAFE_CLOSE_MS = 15000;

const closeWindow = async () => {
  try {
    await getCurrentWindow().close();
  } catch {
    // Ignore close failures in non-Tauri browser contexts.
  }
};

const run = async () => {
  const canvas = document.getElementById('jumpscare');
  if (!(canvas instanceof HTMLCanvasElement)) {
    return;
  }
  const context = canvas.getContext('2d');
  if (!context) {
    return;
  }

  const win = getCurrentWindow();
  const response = await fetch(GIF_SRC);
  const buffer = await response.arrayBuffer();
  const parsed = parseGIF(buffer);
  const frames = decompressFrames(parsed, true) as ParsedFrame[];

  canvas.width = parsed.lsd.width;
  canvas.height = parsed.lsd.height;

  const audio = new Audio(AUDIO_SRC);
  audio.preload = 'auto';
  audio.load();

  await Promise.allSettled([
    win.setIgnoreCursorEvents(true),
    win.setAlwaysOnTop(true),
  ]);

  let frameTimer: number | null = null;
  const clearFrameTimer = () => {
    if (frameTimer !== null) {
      window.clearTimeout(frameTimer);
      frameTimer = null;
    }
  };

  const stopVisual = () => {
    clearFrameTimer();
    context.clearRect(0, 0, canvas.width, canvas.height);
    canvas.style.opacity = '0';
  };

  const drawFrame = (index: number) => {
    const frame = frames[index];
    if (!frame) {
      stopVisual();
      return;
    }

    const imageData = new ImageData(
      // gifuct-js uses Uint8ClampedArray-compatible patches.
      frame.patch as Uint8ClampedArray,
      frame.dims.width,
      frame.dims.height,
    );

    context.putImageData(imageData, frame.dims.left, frame.dims.top);

    const delayMs = Math.max(frame.delay);
    if (index < frames.length - 1) {
      frameTimer = window.setTimeout(() => drawFrame(index + 1), delayMs);
      return;
    }

    frameTimer = null;
    context.clearRect(0, 0, canvas.width, canvas.height);
    canvas.style.opacity = '0';
  };

  canvas.style.opacity = '1';
  await win.show().catch(() => {});
  audio.currentTime = 0;
  void audio.play().catch(() => {});
  drawFrame(0);

  let closed = false;
  const closeOnce = () => {
    if (closed) {
      return;
    }
    closed = true;
    stopVisual();
    void closeWindow();
  };

  audio.addEventListener('ended', closeOnce, { once: true });
  audio.addEventListener('error', closeOnce, { once: true });
  window.setTimeout(closeOnce, FAILSAFE_CLOSE_MS);
};

void run();
