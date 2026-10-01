"""Original 120 BPM score and frame-timed Foley. No samples or network services."""
from functools import lru_cache
from pathlib import Path
from hashlib import sha256
import json
import math
import subprocess

import numpy as np
from scipy import signal
from scipy.io import wavfile

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'out'
SR = 48_000
RNG = np.random.default_rng(7579082)


def clock(duration):
    return np.arange(round(duration * SR), dtype=np.float32) / SR


def hz(note):
    return 440 * 2 ** ((note - 69) / 12)


def add(track, sound, at, gain=1, pan=0):
    start = round(at * SR)
    if start < 0:
        sound = sound[-start:]
        start = 0
    count = min(len(sound), len(track) - start)
    if count <= 0:
        return
    sound = sound[:count]
    if sound.ndim == 1:
        angle = (pan + 1) * math.pi / 4
        track[start:start + count, 0] += sound * (gain * math.cos(angle))
        track[start:start + count, 1] += sound * (gain * math.sin(angle))
    else:
        track[start:start + count] += sound * gain


@lru_cache(maxsize=96)
def tone(note, duration, kind):
    t = clock(duration)
    f = hz(note)
    phase = 2 * np.pi * f * t
    if kind == 'pad':
        env = np.minimum(t / .8, 1) * np.minimum((duration - t) / 1.6, 1)
        channels = []
        for detune in [-.0016, .0016]:
            p = phase * (1 + detune) + .12 * np.sin(t * 2.3)
            y = np.sin(p) + .24 * np.sin(2 * p) + .11 * np.sin(3 * p) + .05 * np.sin(5 * p)
            channels.append(y * env * .7)
        return np.stack(channels, axis=1).astype(np.float32)
    if kind == 'bell':
        y = np.sin(phase + 2.0 * np.exp(-t * 8) * np.sin(phase * 2.003))
        y += .12 * np.sin(phase * 3.99) * np.exp(-t * 5)
        decay = 2.4
    elif kind == 'pluck':
        y = np.sin(phase) + .27 * np.sin(phase * 2.003) + .12 * np.sin(phase * 3.999)
        decay = 5.0
    elif kind == 'bass':
        y = np.sin(phase) + .21 * np.sin(phase * 2) + .09 * np.sin(phase * 3)
        decay = 4.7
    else:
        raise ValueError(f'Unknown instrument: {kind}')
    env = np.minimum(t / .004, 1) * np.exp(-t * decay) * np.minimum((duration - t) / .06, 1)
    return (y * env).astype(np.float32)


def noise(duration, low=250, high=5000):
    n = RNG.standard_normal(round(duration * SR)).astype(np.float32)
    return signal.sosfilt(signal.butter(2, [low, high], btype='bandpass', fs=SR, output='sos'), n).astype(np.float32)


@lru_cache(maxsize=16)
def drum(kind):
    duration = {'kick':.6, 'snare':.25, 'hat':.1, 'openhat':.38}[kind]
    t = clock(duration)
    if kind == 'kick':
        phase = 2 * np.pi * (47 * t + 105 / 29 * (1 - np.exp(-29 * t)))
        y = np.sin(phase) * np.exp(-t * 9) + .13 * noise(duration, 300, 2000) * np.exp(-t * 110)
    elif kind == 'snare':
        y = noise(duration, 850, 8000) * np.exp(-t * 21) + .22 * np.sin(2 * np.pi * 183 * t) * np.exp(-t * 28)
    else:
        y = noise(duration, 6500, 13_000) * np.exp(-t * (65 if kind == 'hat' else 14))
    return (y * np.minimum(t / .001, 1) * np.minimum((duration - t) / .02, 1)).astype(np.float32)


def score(duration):
    music = np.zeros((round(duration * SR), 2), np.float32)
    chords = [(62, 65, 69, 76), (58, 62, 65, 69), (60, 65, 69, 74), (60, 62, 67, 74)]
    roots = [38, 34, 41, 36]
    for at in range(0, 120, 4):
        chord = chords[(at // 4) % 4] if at < 112 else chords[0]
        gain = .039 if 48 <= at < 72 else .028
        for note in chord:
            add(music, tone(note, 6.0, 'pad'), at, gain)
        if at >= 8 and at < 112:
            add(music, tone(chord[0] + 12, 2., 'bell'), at, .052, -.35)
    print('Score: harmony', flush=True)
    for beat in range(16, 224):
        at = beat * .5
        index = int(at // 4) % 4
        strong = 48 <= at < 72 or 96 <= at < 112
        sparse = 38 <= at < 44 or 88 <= at < 94
        if not sparse:
            add(music, drum('kick'), at, .27 if strong else .14)
            if beat % 4 in [1, 3]:
                add(music, drum('snare'), at, .16 if strong else .063, .04)
            add(music, drum('hat'), at + .25, .068 if strong else .032, (-1) ** beat * .35)
            if strong and beat % 4 == 2:
                add(music, drum('openhat'), at + .25, .07, .45)
            add(music, tone(roots[index] + (12 if beat % 8 == 7 else 0), .47, 'bass'), at, .17 if strong else .10)
        chord = chords[index]
        if beat % (2 if sparse else 1) == 0:
            note = chord[[0, 2, 1, 3, 2, 1, 3, 2][beat % 8]] + 12
            add(music, tone(note, 1.2, 'pluck'), at + .25, .074 if strong else .043, math.sin(beat * .7) * .55)
        if strong:
            add(music, tone(chord[(beat + 2) % 4] + 12, .8, 'pluck'), at + .375, .025, .5)
    # A short original D-minor motif follows the visual reveal and returns at the close.
    melody = [74, 77, 81, 79, 77, 74, 72, 69, 74, 77, 84, 81, 79, 77, 76, 74]
    for start, gain in [(48,.084), (64,.055), (96,.07)]:
        for i,note in enumerate(melody):
            add(music,tone(note,1.8,'bell'),start+i*.5,gain,math.sin(i*.9)*.3)
    for i,note in enumerate([62,69,74,77,81,86]):
        add(music,tone(note,2.8,'bell'),112+i*.5,.055,(-1)**i*.3)
    for i in range(16):
        add(music,drum('snare'),44+i*.25,.035+i*.004,0)
    print('Score: rhythm and motif', flush=True)
    # Fixed reflections keep the score wide without washing out interface sounds.
    dry = music.copy()
    for delay,gain in [(.25,.13),(.5,.075),(.75,.04)]:
        n = round(delay * SR)
        music[n:] += dry[:-n, ::-1] * gain
    ir_t = clock(1.6)
    for channel in range(2):
        ir = RNG.standard_normal(len(ir_t)).astype(np.float32) * np.exp(-ir_t*5) * .0018
        ir[:round(.035*SR)] = 0
        music[:,channel] += signal.fftconvolve(dry[:,channel],ir,mode='full')[:len(music)]
    t = clock(duration)
    envelope = np.minimum(t / 2, 1) * np.clip((duration - t) / 3.3, 0, 1)
    envelope *= 1 - .9 * ((t>=90) & (t<90.27))
    music *= envelope[:,None]
    return music


def effect(cue):
    kind = cue['kind']
    note = cue.get('note', 74)
    duration = cue.get('dur', .6)
    at = cue['t']
    gain = cue.get('vel', 1)
    if kind in {'key','stream','click','shutter','paper'}:
        d = .23 if kind=='paper' else .10
        t = clock(d)
        y = noise(d,600,6000)*np.exp(-t*(22 if kind=='paper' else 80))
        y += .17*np.sin(2*np.pi*(430 if kind=='click' else 750)*t)*np.exp(-t*100)
        gain *= {'key':.055,'stream':.028,'click':.21,'shutter':.24,'paper':.15}[kind]
    elif kind in {'bell','blip','tick','check','notice'}:
        if kind=='check': note=74+cue.get('i',0)*2
        if kind=='notice': note=65
        y = tone(note,1.4,'bell').copy()
        if kind=='check': y += tone(note+7,1.4,'bell')*.38
        gain *= {'bell':.15,'blip':.12,'tick':.07,'check':.12,'notice':.10}[kind]
    elif kind in {'settle','land','stamp'}:
        t = clock(.35)
        y = np.sin(2*np.pi*(120*t+.8*(1-np.exp(-t*25))))*np.exp(-t*19)
        y += noise(.35,300,1800)*np.exp(-t*45)*.25
        gain *= .13 if kind!='stamp' else .24
    elif kind=='impact':
        t = clock(2.2)
        y = np.sin(2*np.pi*(37*t+2*(1-np.exp(-t*6))))*np.exp(-t*2.8)
        y += noise(2.2,80,1100)*np.exp(-t*7)*.7
        gain *= .48*cue.get('size',1)
    elif kind in {'whoosh','swoosh','swish','swirl','fly','reverseTo','scan'}:
        if kind=='reverseTo': at -= duration
        t = clock(duration)
        u = t/duration
        envelope = u**2.5 if kind=='reverseTo' else np.sin(np.pi*u)**1.4
        y = noise(duration,250,4200)*envelope
        y += .13*signal.chirp(t,180,duration,1800,method='logarithmic')*envelope
        gain *= .12 if kind=='scan' else .22
    elif kind=='shimmer':
        y = sum(tone(n,duration,'bell') for n in [81,86,89,93])*.25
        gain *= .14
    elif kind=='ocean':
        t = clock(duration)
        y = noise(duration,90,2700)*(.6+.25*np.sin(t*1.2)+.15*np.sin(t*.41))
        y *= np.minimum(t/.8,1)*np.clip((duration-t)/1.5,0,1)
        gain *= .19
    elif kind=='glitch':
        t = clock(.19)
        y = noise(.19,300,4000)*(np.sin(t*190)>0)*np.exp(-t*13)
        gain *= .17
    else:
        raise ValueError(f'Unimplemented sound cue: {kind}')
    y = np.asarray(y, np.float32)
    # All transient endpoints go to zero, including the reverse swell at its edit point.
    fade = min(round(.003*SR), len(y)//2)
    y = y.copy()
    y[:fade] *= np.linspace(0,1,fade,dtype=np.float32)
    y[-fade:] *= np.linspace(1,0,fade,dtype=np.float32)
    pan = cue.get('dir', math.sin(cue.get('i',0)*1.7))*.3
    return at,y,gain,pan


def main():
    data = json.loads((OUT/'timeline.json').read_text())
    duration = data['DURATION']
    assert duration == 123 and data['FPS'] == 60
    music = score(duration)
    foley = np.zeros_like(music)
    duck = np.ones(len(music),np.float32)
    for cue in data['cues']:
        at,y,gain,pan = effect(cue)
        add(foley,y,at,gain,pan)
        if cue['kind'] in {'impact','stamp','notice','click','check'}:
            start = round(cue['t']*SR)
            n = min(round(.35*SR),len(duck)-start)
            duck[start:start+n] *= 1-.15*np.exp(-np.arange(n,dtype=np.float32)/(SR*.09))
    music *= duck[:,None]
    mix = music+foley
    peak = max(float(np.max(np.abs(x))) for x in [music,foley,mix])
    gain = min(1,.89/peak)
    for filename,track in [('music.wav',music),('effects.wav',foley),('mix-input.wav',mix)]:
        assert np.isfinite(track).all()
        wavfile.write(OUT/filename,SR,track*gain)
    print(f'Foley: {len(data["cues"])} cues; peak before mastering {peak*gain:.3f}',flush=True)
    first = subprocess.run(['ffmpeg','-hide_banner','-i',str(OUT/'mix-input.wav'),
        '-af','loudnorm=I=-14:TP=-1.2:LRA=11:print_format=json','-f','null','-'],
        check=True,capture_output=True,text=True)
    (OUT/'audio-analysis.log').write_text(first.stderr)
    stats = json.JSONDecoder().raw_decode(first.stderr[first.stderr.rfind('{'):])[0]
    (OUT/'loudness-analysis.json').write_text(json.dumps(stats,indent=2)+'\n')
    filters = ('loudnorm=I=-14:TP=-1.2:LRA=11:linear=true:'
        f'measured_I={stats["input_i"]}:measured_TP={stats["input_tp"]}:'
        f'measured_LRA={stats["input_lra"]}:measured_thresh={stats["input_thresh"]}:'
        f'offset={stats["target_offset"]}:print_format=json')
    second = subprocess.run(['ffmpeg','-hide_banner','-y','-i',str(OUT/'mix-input.wav'),
        '-af',filters,'-ar',str(SR),'-ac', '2','-c:a','pcm_s24le',str(OUT/'master.wav')],
        check=True,capture_output=True,text=True)
    (OUT/'audio-mastering.log').write_text(second.stderr)
    result = json.JSONDecoder().raw_decode(second.stderr[second.stderr.rfind('{'):])[0]
    (OUT/'loudness-mastering.json').write_text(json.dumps(result,indent=2)+'\n')
    provenance = {'timelineSha256': sha256((OUT/'timeline.json').read_bytes()).hexdigest(),
        'scriptSha256': sha256(Path(__file__).read_bytes()).hexdigest()}
    (OUT/'audio-source.json').write_text(json.dumps(provenance,indent=2)+'\n')
    print(json.dumps(result),flush=True)


if __name__ == '__main__':
    main()
