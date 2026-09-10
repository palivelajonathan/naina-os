# NAINA OS — Voice Runtime Package (`voice-runtime`)

The `voice-runtime` package is responsible for supervising voice processing, speech-to-text (STT) transcription via Whisper, and text-to-speech (TTS) synthesis via Piper in NAINA OS.

## DAG Position & Architectural Constraints

```
apps → voice-runtime → (runtime, services) → (kernel, capabilities, configuration, logging)
```

### Allowed Direct Dependencies:
- `runtime`
- `services`
- `configuration`
- `logging`

### Forbidden Dependencies:
- `orchestrator`
- `model-providers`
- `model-runtime`
- `context-engine`
- `memory`
- `tool-registry`
- `browser-runtime`
- `desktop-runtime`
- `automation`
- `sdk`
- `ui`
- `kernel`
- `capabilities`
- `event-bus`

## First Alpha Goals
- Capture microphone audio stream (16kHz 16-bit mono PCM).
- Transcribe audio to text via Whisper STT model.
- Synthesize assistant text output into spoken audio via Piper TTS engine.
- Enforce strict voice-to-voice turn latency budget of `< 700 ms`.
