# desk-talk
 
Transcription for your desktop.

A software that records when you press a button down, and types what you said when you release it.

## Quickstart

Assign your OpenAI API key to the `OPENAI_API_KEY` environment variable and run:

```
desk-talk --ptt-key scroll-lock
```

Or pass your OpenAI API key as a flag like so:

```
desk-talk --ptt-key scroll-lock --api-key [YOUR_API_KEY]
```

## Other Usage

### Special Keys

For 

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


