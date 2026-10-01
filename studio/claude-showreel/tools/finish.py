"""Mux the checked picture and original mix, decode the result, and package sources."""
from hashlib import sha256
from pathlib import Path
import json
import subprocess
import zipfile

import numpy as np
from scipy.io import wavfile

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'out'
NAME = 'A4N_从一行字到一整个世界_1080p60.mp4'


def run(args):
    return subprocess.run(args, check=True, capture_output=True, text=True)


def probe(file):
    return json.loads(run(['ffprobe', '-v', 'error', '-show_streams', '-show_format',
        '-show_chapters', '-of', 'json', str(file)]).stdout)


def check_sources():
    timeline = json.loads((OUT/'timeline.json').read_text())
    picture = json.loads((OUT/'picture.json').read_text())
    audio = json.loads((OUT/'audio-source.json').read_text())
    assert picture['frames'] == timeline['DURATION'] * timeline['FPS'] == 7380
    assert picture['from'] == 0 and picture['to'] == timeline['DURATION']
    h = sha256(json.dumps({'from': 0, 'to': 7380, 'FPS': 60}, separators=(',', ':')).encode())
    for filename in picture['source']:
        h.update(filename.encode())
        h.update((ROOT/filename).read_bytes())
    assert h.hexdigest() == picture['signature'], 'Picture source changed after rendering'
    assert audio['timelineSha256'] == sha256((OUT/'timeline.json').read_bytes()).hexdigest(), 'Audio timeline changed'
    assert audio['scriptSha256'] == sha256((ROOT/'tools/audio.py').read_bytes()).hexdigest(), 'Audio source changed'
    assert (OUT/'review/source-check.json').is_file(), 'Run npm test first'
    stems = {}
    for name in ['music.wav', 'effects.wav', 'master.wav']:
        sr, x = wavfile.read(OUT/name)
        assert sr == 48000 and x.shape == (123*48000, 2), (name, sr, x.shape)
        if np.issubdtype(x.dtype, np.integer):
            x = x.astype(np.float64) / (np.iinfo(x.dtype).max + 1)
        assert np.isfinite(x).all()
        peak = float(np.abs(x).max())
        rms = float(np.sqrt(np.mean(x.astype(np.float64)**2)))
        assert .01 < peak < 1 and rms > .001, (name, peak, rms)
        stems[name] = {'sampleRate': sr, 'frames': len(x), 'channels': 2, 'peak': peak, 'rms': rms}
    return timeline, picture, stems


def contact_sheet(timeline):
    folder = OUT/'review/decoded'
    folder.mkdir(parents=True, exist_ok=True)
    # Pick settled teaching moments, rather than half-typed text or the closing fade.
    times = [4.3,10.8,16.5,25.7,36.4,45.8,50.5,54.8,58.6,64.5,71.2,78.7,85.5,93.25,97.2,101.5,106.7,111,119]
    assert len(times) == len(timeline['chapters'])
    assert all(c['start'] <= t < c['end'] for t,c in zip(times,timeline['chapters']))
    frames = [round(t*60) for t in times]
    select = '+'.join(f'eq(n,{n})' for n in frames)
    run(['ffmpeg','-hide_banner','-loglevel','error','-y','-threads','2','-i',str(OUT/NAME),
        '-vf',f"select='{select}',scale=640:360",'-fps_mode','vfr','-q:v','2','-start_number','0',str(folder/'%02d.jpg')])
    montage = ['montage']
    for i,t in enumerate(times):
        montage += ['-label',f'{t:.2f} s',str(folder/f'{i:02d}.jpg')]
    run(montage+['-tile','3x','-geometry','640x360+4+4','-background','#171310',
        '-fill','#d7bf96','-font','DejaVu-Sans','-pointsize','16','-quality','94',str(OUT/'storyboard.jpg')])
    run(['ffmpeg','-hide_banner','-loglevel','error','-y','-ss','50','-i',str(OUT/NAME),
        '-frames:v','1','-q:v','2',str(OUT/'poster.jpg')])


def main():
    timeline, picture, stems = check_sources()
    metadata = [';FFMETADATA1','title=A4N · 从一行字，到一整个世界','artist=A4N',
        'comment=123 秒原创动画；界面与生成画面为动画示意；原创音乐与音效']
    for c in timeline['chapters']:
        metadata += ['[CHAPTER]','TIMEBASE=1/1000',f'START={round(c["start"]*1000)}',
            f'END={round(c["end"]*1000)}',f'title={c["title"]}']
    (OUT/'chapters.ffmetadata').write_text('\n'.join(metadata)+'\n')
    partial = OUT/NAME.replace('.mp4','.partial.mp4')
    run(['ffmpeg','-hide_banner','-loglevel','error','-y','-i',str(OUT/'picture.mp4'),
        '-i',str(OUT/'master.wav'),'-f','ffmetadata','-i',str(OUT/'chapters.ffmetadata'),
        '-map','0:v:0','-map','1:a:0','-map_metadata','2','-map_chapters','2',
        '-c:v','copy','-c:a','aac','-b:a','320k','-ar','48000','-ac','2',
        '-t','123','-movflags','+faststart',str(partial)])
    partial.replace(OUT/NAME)
    info = probe(OUT/NAME)
    video = next(s for s in info['streams'] if s['codec_type']=='video')
    audio = next(s for s in info['streams'] if s['codec_type']=='audio')
    assert (video['width'],video['height'],video['r_frame_rate'],int(video['nb_frames'])) == (1920,1080,'60/1',7380)
    assert video['codec_name']=='h264' and video['pix_fmt']=='yuv420p'
    assert audio['codec_name']=='aac' and int(audio['sample_rate'])==48000 and audio['channels']==2
    assert abs(float(info['format']['duration'])-123)<.05
    assert abs(float(video['duration'])-float(audio['duration']))<.03
    assert len(info['chapters'])==len(timeline['chapters'])
    decode = run(['ffmpeg','-hide_banner','-loglevel','error','-xerror','-threads','2','-i',str(OUT/NAME),
        '-map','0:v:0','-map','0:a:0','-f','null','-'])
    assert not decode.stderr.strip(), decode.stderr
    loud = run(['ffmpeg','-hide_banner','-nostats','-i',str(OUT/NAME),'-vn',
        '-af','loudnorm=I=-14:TP=-1.2:LRA=11:print_format=json','-f','null','-'])
    loudness = json.JSONDecoder().raw_decode(loud.stderr[loud.stderr.rfind('{'):])[0]
    assert abs(float(loudness['input_i'])+14)<.7,loudness
    assert float(loudness['input_tp'])<=-.5,loudness
    contact_sheet(timeline)
    report = {'file':NAME,'sha256':sha256((OUT/NAME).read_bytes()).hexdigest(),
        'durationSeconds':float(info['format']['duration']),'width':1920,'height':1080,'fps':60,'frames':7380,
        'videoCodec':'h264','audioCodec':'aac','sampleRate':48000,'channels':2,
        'chapterCount':len(timeline['chapters']),'soundCueCount':len(timeline['cues']),
        'decodedEntireFile':True,'integratedLUFS':float(loudness['input_i']),
        'truePeakDBTP':float(loudness['input_tp']),'stems':stems,'pictureSignature':picture['signature']}
    (OUT/'validation.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps(report,ensure_ascii=False),flush=True)
    package_sources()


def package_sources():
    files = [ROOT/p for p in ['README.md','requirements.txt','package.json','package-lock.json','index.html','watch.html']]
    for folder in ['src','tools','assets']:
        files += [p for p in (ROOT/folder).rglob('*') if p.is_file() and '__pycache__' not in p.parts]
    files += [OUT/p for p in ['music.wav','effects.wav','master.wav','timeline.json','audio-source.json',
        'loudness-mastering.json','validation.json','storyboard.jpg','poster.jpg','review/source-check.json']]
    if (OUT/'review/playback.json').is_file():
        files.append(OUT/'review/playback.json')
    package = OUT/'A4N_动画工程与分轨.zip'
    partial = package.with_suffix('.partial.zip')
    with zipfile.ZipFile(partial,'w',compression=zipfile.ZIP_DEFLATED,compresslevel=5) as archive:
        for file in sorted(files): archive.write(file,file.relative_to(ROOT))
        archive.write(ROOT.parents[1]/'LICENSE','LICENSE')
    partial.replace(package)
    print(package,flush=True)


if __name__ == '__main__':
    main()
