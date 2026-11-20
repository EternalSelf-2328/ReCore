# ReCord sound packs

These are the sound themes from [leriart/ReCord](https://github.com/leriart/ReCord),
bundled with ReCore and decoded by `src/sounds.rs`.

Each folder has the same twenty-two short WAVs, named after what ReCore plays:

- call and message sounds: `join`, `leave`, `mute`, `unmute`, `deafen`,
  `undeafen`, `message`, `mention`, `viewer-join`, `viewer-leave`,
  `stream-on`, `stream-off`;
- interface sounds: `hover`, `select`, `back`, `open`, `close`, `navigate`,
  `toggle`, `send`, `type`, `boot`.

ReCord ships them as Ogg Opus (and Kenney's sets as WAV). They are converted
here to mono 24 kHz 16-bit PCM WAV, because ReCore's sound player reads WAV
without a codec dependency. The conversion only changes the container, sample
rate and channel count; the audio is unchanged.

| Folder | Source | Licence |
| --- | --- | --- |
| `kenney`, `kenney-interface` | Kenney's UI SFX and Interface Sounds, by Kenney Vleugels ([kenney.nl](https://kenney.nl)) | CC0 1.0 |
| `arcade` … `zen` | UI SFX, by Yuki Capital ([romainsimon/uisfx](https://github.com/romainsimon/uisfx)) | CC0 1.0 |
| the terminal set in `../record` | ReCord's original signal pack | CC0 1.0 |

The licence texts are in this folder. ReCord's theme code is Apache-2.0 and
its companion plugin is GPL-3.0-or-later; the sounds themselves are CC0 1.0.
