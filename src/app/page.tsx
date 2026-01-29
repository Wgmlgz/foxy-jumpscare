"use client";

import { decompressFrames, parseGIF, type ParsedFrame } from "gifuct-js";
import { useEffect, useRef, useState } from "react";

const GIF_SRC = "/media/fnaf-2-fnaf.gif";
const AUDIO_SRC = "/media/scream.mp3";
const INTERVAL_MS = 5000;
const MIN_FRAME_DELAY_MS = 20;

export default function Home() {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const framesRef = useRef<ParsedFrame[]>([]);
  const isPlayingRef = useRef(false);
  const frameTimerRef = useRef<number | null>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const [ready, setReady] = useState(false);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const enableClickThrough = async () => {
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        await getCurrentWindow().setIgnoreCursorEvents(true);
        await getCurrentWindow().setAlwaysOnTop(true);
      } catch {
        // Ignore failures when not running inside Tauri.
      }
    };

    enableClickThrough();
  }, []);

  useEffect(() => {
    let cancelled = false;

    const loadGif = async () => {
      const response = await fetch(GIF_SRC);
      const buffer = await response.arrayBuffer();
      const parsed = parseGIF(buffer);
      const frames = decompressFrames(parsed, true);

      if (cancelled) {
        return;
      }

      framesRef.current = frames;

      const canvas = canvasRef.current;
      if (canvas) {
        canvas.width = parsed.lsd.width;
        canvas.height = parsed.lsd.height;
      }

      setReady(true);
    };

    loadGif().catch(() => {
      // If the GIF fails to load, leave the screen blank.
    });

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!ready) {
      return;
    }

    const canvas = canvasRef.current;
    const context = canvas?.getContext("2d");
    if (!canvas || !context) {
      return;
    }

    const audio = new Audio(AUDIO_SRC);
    audio.preload = "auto";
    audioRef.current = audio;

    const clearFrameTimer = () => {
      if (frameTimerRef.current !== null) {
        window.clearTimeout(frameTimerRef.current);
        frameTimerRef.current = null;
      }
    };

    const stopPlayback = (shouldClose: boolean) => {
      clearFrameTimer();
      audio.pause();
      context.clearRect(0, 0, canvas.width, canvas.height);
      setVisible(false);
      isPlayingRef.current = false;

      if (shouldClose) {
        const closeApp = async () => {
          try {
            const { getCurrentWindow } = await import("@tauri-apps/api/window");
            await getCurrentWindow().close();
          } catch {
            // Ignore failures when not running inside Tauri.
          }
        };

        closeApp();
      }
    };

    const drawFrame = (index: number) => {
      const frame = framesRef.current[index];
      if (!frame) {
        stopPlayback(true);
        return;
      }

      const imageData = new ImageData(
        // eslint-disable-next-line @typescript-eslint/ban-ts-comment
        // @ts-ignore
        frame.patch,
        frame.dims.width,
        frame.dims.height
      );

      context.putImageData(imageData, frame.dims.left, frame.dims.top);

      const delayMs = Math.max(frame.delay);

      if (index < framesRef.current.length - 1) {
        frameTimerRef.current = window.setTimeout(
          () => drawFrame(index + 1),
          delayMs
        );
      } else {
        frameTimerRef.current = null;
        context.clearRect(0, 0, canvas.width, canvas.height);

      }
    };

    const playOnce = () => {
      if (isPlayingRef.current || framesRef.current.length === 0) {
        return;
      }

      isPlayingRef.current = true;
      setVisible(true);
      context.clearRect(0, 0, canvas.width, canvas.height);

      audio.currentTime = 0;
      audio.play().catch(() => {
        // Autoplay may be blocked in browsers; desktop shells usually allow it.
      });

      drawFrame(0);
    };

    const handleAudioEnded = () => {
      stopPlayback(true);
    };

    const handleAudioError = () => {
      stopPlayback(true);
    };

    audio.addEventListener("ended", handleAudioEnded);
    audio.addEventListener("error", handleAudioError);

    playOnce();
    const intervalId = window.setInterval(playOnce, INTERVAL_MS);

    return () => {
      window.clearInterval(intervalId);
      audio.removeEventListener("ended", handleAudioEnded);
      audio.removeEventListener("error", handleAudioError);
      stopPlayback(false);
    };
  }, [ready]);

  return (
    <div className="fixed inset-0 flex items-center justify-center bg-transparent pointer-events-none">
      <canvas
        ref={canvasRef}
        className={`h-full w-full object-contain ${visible ? "opacity-100" : "opacity-0"
          }`}
      />
    </div>
  );
}
