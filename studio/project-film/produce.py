"""Original A4N film: speech, synthesized score/Foley, deterministic browser render."""

import argparse
import asyncio
import base64
import functools
import hashlib
import http.server
import json
import math
from pathlib import Path
import subprocess
import threading
import wave

import numpy as np

ROOT = Path(__file__).resolve().parent
OUT = ROOT / "out"
SCRIPT = json.loads((ROOT / "script.json").read_text())
SR = 48000
DURATION = SCRIPT["duration"]


def run(*args, **kwargs):
    return subprocess.run(args, check=True, **kwargs)


def pcm(path):
    data = subprocess.check_output([
        "ffmpeg", "-v", "error", "-i", str(path), "-f", "f32le",
        "-ac", "1", "-ar", str(SR), "pipe:1",
    ])
    return np.frombuffer(data, np.float32).copy()


def wav(path, data):
    if not np.isfinite(data).all() or np.max(np.abs(data)) > 1:
        raise ValueError(f"Invalid or clipped PCM: {path}")
    with wave.open(str(path), "wb") as output:
        output.setnchannels(1 if data.ndim == 1 else data.shape[1])
        output.setsampwidth(2)
        output.setframerate(SR)
        output.writeframes((data * 32767).astype("<i2").tobytes())


def stamp(seconds):
    ms = round(seconds * 1000)
    return f"{ms // 3600000:02}:{ms // 60000 % 60:02}:{ms // 1000 % 60:02},{ms % 1000:03}"


async def speech():
    import edge_tts

    OUT.mkdir(exist_ok=True)
    subtitles = []
    voice = np.zeros(int(DURATION * SR), np.float32)
    for index, scene in enumerate(SCRIPT["scenes"]):
        text = "".join(scene["lines"])
        # Content-address the cache so a script edit cannot reuse stale speech.
        digest = hashlib.sha256((SCRIPT["voice"] + text + "+4%").encode()).hexdigest()[:12]
        path = OUT / f"voice-{index:02}-{digest}.mp3"
        timing = path.with_suffix(".json")
        if not path.exists() or not timing.exists():
            boundaries = []
            temporary = path.with_suffix(".part")
            try:
                with temporary.open("wb") as f:
                    stream = edge_tts.Communicate(text, SCRIPT["voice"], rate="+4%", boundary="WordBoundary")
                    async for item in stream.stream():
                        if item["type"] == "audio":
                            f.write(item["data"])
                        elif item["type"] == "WordBoundary":
                            boundaries.append({k: item[k] for k in ("offset", "duration", "text")})
                if not boundaries or temporary.stat().st_size == 0:
                    raise RuntimeError(f"Speech returned no audio/timing for scene {index}")
                temporary.replace(path)
                timing.write_text(json.dumps(boundaries, ensure_ascii=False))
            finally:
                temporary.unlink(missing_ok=True)
        boundaries = json.loads(timing.read_text())
        audio = pcm(path)
        start = scene["start"] + (0.65 if index < 10 else 0.2)
        available = scene["end"] - start - 0.25
        duration = len(audio) / SR
        speed = max(1.0, duration / available)
        if speed > 1.22:
            raise ValueError(f"Narration too dense in {scene['id']}: {duration:.1f}s / {available:.1f}s")
        if speed > 1:
            raw = subprocess.check_output([
                "ffmpeg", "-v", "error", "-i", str(path), "-af", f"atempo={speed}",
                "-f", "f32le", "-ar", str(SR), "-ac", "1", "pipe:1",
            ])
            audio = np.frombuffer(raw, np.float32).copy()
        # Boundary matching uses spoken alphanumerics/CJK, ignoring punctuation.
        word_positions = []
        count = 0
        for boundary in boundaries:
            count += sum(char.isalnum() for char in boundary["text"])
            word_positions.append((count, boundary["offset"] / 1e7 / speed))
        consumed = 0
        for j, line in enumerate(scene["lines"]):
            line_start = next((offset for count, offset in word_positions if count > consumed), 0)
            consumed += sum(char.isalnum() for char in line)
            line_end = next((offset for count, offset in word_positions if count > consumed), len(audio) / SR)
            subtitles.append({"start": start + line_start, "end": start + line_end, "text": line})
        peak = float(np.max(np.abs(audio)))
        if peak < 0.001:
            raise ValueError(f"Silent speech: {path}")
        audio *= 0.64 / peak
        offset = round(start * SR)
        voice[offset:offset + len(audio)] += audio
        print(f"voice {index + 1:02}: {len(audio) / SR:.2f}s, speed {speed:.3f}", flush=True)
    wav(OUT / "narration.wav", voice)
    (OUT / "subtitles.json").write_text(json.dumps(subtitles, ensure_ascii=False, indent=2))
    (OUT / "A4N_中文字幕.srt").write_text("\n\n".join(
        f"{i + 1}\n{stamp(cue['start'])} --> {stamp(cue['end'])}\n{cue['text']}"
        for i, cue in enumerate(subtitles)
    ) + "\n")


def add(track, signal, start, gain=1, pan=0):
    offset = round(start * SR)
    if offset < 0:
        signal = signal[-offset:]
        offset = 0
    length = min(len(signal), len(track) - offset)
    if length <= 0:
        return
    angle = (pan + 1) * math.pi / 4
    track[offset:offset + length, 0] += signal[:length] * (gain * math.cos(angle))
    track[offset:offset + length, 1] += signal[:length] * (gain * math.sin(angle))


def tone(midi, duration, kind="bell"):
    t = np.arange(round(duration * SR), dtype=np.float32) / SR
    frequency = 440 * 2 ** ((midi - 69) / 12)
    phase = 2 * np.pi * frequency * t
    if kind == "pad":
        sound = (np.sin(phase) + 0.32 * np.sin(phase * 1.003) + 0.18 * np.sin(phase * 2)) / 1.5
        envelope = np.minimum(t / 0.8, 1) * np.minimum((duration - t) / 1.8, 1)
        return sound * envelope * (0.92 + 0.08 * np.sin(t * 2))
    if kind == "bass":
        return (np.sin(phase) + 0.15 * np.sin(phase * 2)) * np.minimum(t / 0.018, 1) * np.exp(-t * 2.5)
    return (np.sin(phase + 1.5 * np.sin(phase * 2) * np.exp(-t * 7))
            + 0.2 * np.sin(phase * 3)) * np.exp(-t * 2.2) * np.minimum(t / 0.009, 1)


def score():
    rng = np.random.default_rng(4701)
    size = int(DURATION * SR)
    music = np.zeros((size, 2), np.float32)
    effects = np.zeros_like(music)
    # Original D-minor/add9 progression; pads, FM keys, arpeggios and soft percussion.
    chords = [[50, 57, 60, 64, 65], [46, 53, 57, 60, 62], [41, 53, 57, 60, 67], [48, 55, 60, 62, 64]]
    for bar in range(30):
        start = bar * 4
        chord = chords[bar % 4]
        for k, pitch in enumerate(chord):
            add(music, tone(pitch, 5.8, "pad"), start, 0.027, (k - 2) / 3)
        for beat in range(8):
            time = start + beat * 0.5
            intensity = 0.45 if time < 8 or time > 113 else (0.85 if time < 69 else 1.0)
            add(music, tone(chord[0] - 12, 0.55, "bass"), time, 0.10 * intensity)
            if 8 <= time < 114:
                t = np.arange(int(SR * 0.22), dtype=np.float32) / SR
                kick = np.sin(2 * np.pi * (45 * t + 8 * (1 - np.exp(-t * 28)))) * np.exp(-t * 22)
                if beat % 2 == 0:
                    add(music, kick, time, 0.11 * intensity)
                else:
                    noise = rng.normal(size=len(t)).astype(np.float32)
                    noise = noise - np.roll(noise, 1)
                    add(music, noise * np.exp(-t * 38), time, 0.019 * intensity, 0.15)
            for half in range(2):
                when = time + half * 0.25
                pitch = chord[(beat * 2 + half) % 5] + 12
                add(music, tone(pitch, 1.45), when, 0.027 * intensity, math.sin(when * 0.9) * 0.65)
        melody = [74, 77, 81, 79, 77, 76, 72, 74]
        if bar % 2 == 0:
            for k in range(4):
                add(music, tone(melody[(bar // 2 + k) % 8], 2.4), start + k * 0.85, 0.034, -0.2)
    # Short stereo delays create space without imported samples or an effects library.
    delay = round(0.375 * SR)
    music[delay:, 0] += music[:-delay, 1].copy() * 0.20
    music[delay:, 1] += music[:-delay, 0].copy() * 0.17
    for i, scene in enumerate(SCRIPT["scenes"]):
        when = scene["start"]
        t = np.arange(round(1.1 * SR), dtype=np.float32) / SR
        noise = rng.normal(0, 1, len(t)).astype(np.float32)
        noise = np.convolve(noise, np.ones(21, np.float32) / 21, mode="same")
        whoosh = noise * np.sin(np.pi * t / 1.1) ** 3
        add(effects, whoosh, when - 0.65, 0.13, -0.55 if i % 2 else 0.55)
        add(effects, tone(74 if i % 2 else 81, 1.8), when + 0.06, 0.07, 0.2)
    # Interaction cues: save/test/default, mode choice, blueprint lock, scene beats.
    for when in [23.8, 26.0, 28.1, 35.2, 38.4, 47, 49.5, 52, 60, 63.5, 72, 75, 78, 85, 87.5, 90, 98, 100, 102, 109, 111]:
        t = np.arange(round(0.065 * SR), dtype=np.float32) / SR
        click = np.sin(2 * np.pi * 1550 * t) * np.exp(-t * 105)
        add(effects, click, when, 0.095, math.sin(when) * 0.45)
    # Final resolving shimmer and low breath.
    for k, pitch in enumerate([50, 57, 62, 65, 69, 76]):
        add(music, tone(pitch, 4.8), 115 + k * 0.045, 0.045, (k - 2.5) / 4)
    timeline = np.arange(size, dtype=np.float32) / SR
    fade = np.minimum(timeline / 1.4, 1) * np.clip((120 - timeline) / 2.2, 0, 1)
    music *= fade[:, None]
    effects *= fade[:, None]
    if max(np.max(np.abs(music)), np.max(np.abs(effects))) >= 0.98:
        raise ValueError("Unexpected score peak")
    wav(OUT / "music.wav", music)
    wav(OUT / "effects.wav", effects)
    run("ffmpeg", "-y", "-v", "warning", "-i", str(OUT / "music.wav"),
        "-i", str(OUT / "narration.wav"), "-i", str(OUT / "effects.wav"),
        "-filter_complex",
        "[1:a]asplit=2[voice][control];"
        "[0:a][control]sidechaincompress=threshold=0.04:ratio=3:attack=35:release=420[duck];"
        "[duck][voice][2:a]amix=inputs=3:normalize=0,"
        "loudnorm=I=-16:TP=-1.5:LRA=9:linear=false[a]",
        "-map", "[a]", "-ar", str(SR), "-ac", "2", str(OUT / "master.wav"))
    print("Original music, effects and voice mixed.", flush=True)


class QuietHandler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *args):
        pass


def render(preview=False):
    from PIL import Image, ImageDraw, ImageFont
    from playwright.sync_api import sync_playwright

    handler = functools.partial(QuietHandler, directory=str(ROOT))
    with http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler) as server:
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            with sync_playwright() as playwright:
                browser = playwright.chromium.launch(executable_path="/usr/bin/chromium", args=["--disable-dev-shm-usage"])
                page = browser.new_page(viewport={"width": 1920, "height": 1080}, device_scale_factor=1)
                errors = []
                page.on("pageerror", lambda error: errors.append(str(error)))
                page.goto(f"http://127.0.0.1:{server.server_port}/film.html?render=1")
                page.wait_for_function("window.filmReady === true")
                samples = [4.5, 12, 25, 39.5, 50, 63, 76, 89, 101, 111, 117]
                sheet = Image.new("RGB", (1440, 4 * 295), "#0a1018")
                label_font = ImageFont.truetype("/usr/share/fonts/TTF/DejaVuSans.ttf", 19)
                draw = ImageDraw.Draw(sheet)
                for index, second in enumerate(samples):
                    page.evaluate("t => window.drawFrame(t)", second)
                    frame = OUT / f"frame-{second:05.1f}.png"
                    page.screenshot(path=str(frame))
                    shot = Image.open(frame).resize((480, 270))
                    x, y = index % 3 * 480, index // 3 * 295
                    sheet.paste(shot, (x, y))
                    draw.text((x + 12, y + 273), f"{second:05.1f}s / {SCRIPT['scenes'][index]['id']}", font=label_font, fill="#d9d6ce")
                sheet.save(OUT / "storyboard.jpg", quality=94)
                if errors:
                    raise RuntimeError("Browser error: " + "; ".join(errors))
                if preview:
                    browser.close()
                    return
                frames = round(DURATION * SCRIPT["fps"])
                command = [
                    "ffmpeg", "-y", "-v", "warning", "-f", "image2pipe", "-vcodec", "mjpeg",
                    "-framerate", str(SCRIPT["fps"]), "-i", "pipe:0", "-i", str(OUT / "master.wav"),
                    "-i", str(OUT / "A4N_中文字幕.srt"), "-map", "0:v", "-map", "1:a", "-map", "2:s",
                    "-c:v", "libx264", "-preset", "fast", "-crf", "17", "-threads", "3",
                    "-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "256k", "-c:s", "mov_text",
                    "-metadata:s:s:0", "language=zho", "-disposition:s:0", "0",
                    "-metadata", "title=" + SCRIPT["title"], "-movflags", "+faststart",
                    "-t", str(DURATION), str(OUT / "A4N_项目介绍_1080p.mp4"),
                ]
                encoder = subprocess.Popen(command, stdin=subprocess.PIPE)
                try:
                    for frame in range(frames):
                        data = page.evaluate("t => { drawFrame(t); return document.querySelector('canvas').toDataURL('image/jpeg', 0.97).split(',')[1]; }", frame / SCRIPT["fps"])
                        encoder.stdin.write(base64.b64decode(data))
                        if frame % 150 == 0:
                            print(f"render {frame}/{frames} ({frame / frames:.0%})", flush=True)
                    encoder.stdin.close()
                    if encoder.wait() != 0:
                        raise RuntimeError("Video encoder failed")
                except BaseException:
                    encoder.kill()
                    encoder.wait()
                    raise
                finally:
                    browser.close()
                if errors:
                    raise RuntimeError("Browser error: " + "; ".join(errors))
        finally:
            server.shutdown()


def check():
    scenes = SCRIPT["scenes"]
    assert scenes[0]["start"] == 0 and scenes[-1]["end"] == DURATION
    assert all(a["end"] == b["start"] for a, b in zip(scenes, scenes[1:]))
    cues = json.loads((OUT / "subtitles.json").read_text())
    assert all(0 <= cue["start"] < cue["end"] <= DURATION for cue in cues)
    assert all(a["end"] <= b["start"] + 0.001 for a, b in zip(cues, cues[1:]))
    assert [c["text"] for c in cues] == [line for scene in scenes for line in scene["lines"]]
    for name in ["music.wav", "effects.wav", "narration.wav", "master.wav"]:
        with wave.open(str(OUT / name)) as stream:
            assert stream.getnframes() == DURATION * SR
            data = np.frombuffer(stream.readframes(stream.getnframes()), "<i2")
            assert 200 < np.max(np.abs(data.astype(np.int32))) < 32767
    video = OUT / "A4N_项目介绍_1080p.mp4"
    if video.exists():
        report = json.loads(subprocess.check_output([
            "ffprobe", "-v", "error", "-show_streams", "-show_format", "-of", "json", str(video),
        ]))
        picture = next(s for s in report["streams"] if s["codec_type"] == "video")
        sound = next(s for s in report["streams"] if s["codec_type"] == "audio")
        assert (picture["width"], picture["height"]) == (1920, 1080)
        assert picture["avg_frame_rate"] == "30/1" and int(picture["nb_frames"]) == 3600
        assert sound["channels"] == 2 and sound["sample_rate"] == "48000"
        assert abs(float(report["format"]["duration"]) - DURATION) < 0.05
        run("ffmpeg", "-v", "error", "-i", str(video), "-f", "null", "-")
        (OUT / "validation.json").write_text(json.dumps(report, ensure_ascii=False, indent=2))
    print("PASS: scene continuity, speech/subtitle alignment, audio peaks/durations, final AV metadata/decode if present.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=["audio", "preview", "render", "check"])
    action = parser.parse_args().action
    OUT.mkdir(exist_ok=True)
    if action == "audio":
        asyncio.run(speech())
        score()
    elif action in ("preview", "render"):
        render(preview=action == "preview")
    else:
        check()
