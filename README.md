# desk-talk
 
Transcription for your desktop.

A software that records what you say when you press a button down, and types what you said when you release it.

> [!IMPORTANT]
> ⚠️ This video contains sound and is intended to be listened to with audio on. ⚠️

https://github.com/sloganking/desk-talk/assets/16965931/e5da605b-3a9d-4394-b4ec-a3de65605a65

## Setup

Make sure [ffmpeg](https://www.ffmpeg.org/) is installed and added to your PATH

## Quickstart

Assign your OpenAI API key to the `OPENAI_API_KEY` environment variable and run:

```
desk-talk --ptt-key scroll-lock
```

Or pass your OpenAI API key as a flag like so:

```
desk-talk --ptt-key scroll-lock --api-key [YOUR_API_KEY]
```

`desk-talk` will now record every time you hold down the ptt-key, and type what you spoke every time you release it.

> [!NOTE]
> 
> You can manage your OpenAI API keys at https://platform.openai.com/api-keys
## Other Usage

### Special Keys

To find the name of a key by pressing it, run:

```
desk-talk show-key-presses
```

If your key shows as `Unknown(number)`, pass `number` to the `--special-ptt-key` flag like so:

```
desk-talk --special-ptt-key 125
```

### Non-default recording device

To use a microphone other than the system default, run


```
desk-talk list-devices
```

to get a list of system microphone names. And pass the desired microphone name to ``--device`` like so:


```
desk-talk --ptt-key scroll-lock --device "Microphone (3- USB Audio Device)"
```

### Local transcription

To run transcription locally without the OpenAI API, specify a model size with
`--model` and pass the `--local` flag:

```
desk-talk --ptt-key scroll-lock --local --model tiny
```

Available models include `tiny`, `base`, `small`, `medium`, and the large
variants `large-v1`, `large-v2`, or `large-v3`.


