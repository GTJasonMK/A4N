import { useEffect, useMemo } from "react";
import { useStore } from "zustand";
import { createStore } from "zustand/vanilla";

interface Playback {
  time: number;
  playing: boolean;
  seek: (time: number) => void;
  pause: () => void;
  toggle: () => void;
}

export type DirectorPlayback = ReturnType<typeof createPlayback>;

function createPlayback() {
  return createStore<Playback>((set) => ({
    time: 0, playing: false,
    seek: (time) => set({ time, playing: false }),
    pause: () => set({ playing: false }),
    toggle: () => set((state) => ({ playing: !state.playing })),
  }));
}

export function useDirectorPlayback(total: number, scope: string): DirectorPlayback {
  const clock = useMemo(createPlayback, [scope]);
  const playing = useStore(clock, (state) => state.playing);
  useEffect(() => {
    if (clock.getState().time > total) clock.getState().seek(total);
    if (!playing || total <= 0) return;
    if (clock.getState().time >= total) clock.setState({ time: 0 });
    const origin = clock.getState().time;
    const started = performance.now();
    let frame = 0;
    const tick = (now: number) => {
      const time = Math.min(total, origin + now - started);
      clock.setState({ time, playing: time < total });
      if (time < total) frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    const hide = () => { if (document.hidden) clock.getState().pause(); };
    document.addEventListener("visibilitychange", hide);
    return () => { cancelAnimationFrame(frame); document.removeEventListener("visibilitychange", hide); };
  }, [clock, playing, total]);
  return clock;
}
