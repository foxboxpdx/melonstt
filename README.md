# MelonSTT
Speech-to-Text to VRC via OSC

## About
MelonSTT is a simple speech-to-text transcriber program for Windows utilizing the whisper-rs Rust bindings to whisper.cpp for the transcription, and slint for the front-end.  It is specifically intended to be used with VRChat via some manner of in-game desktop interface (OVR Toolkit, Desktop++, etc) - the user simply clicks on one of the record buttons, speaks into their microphone, reviews the transcribed text, and clicks the 'send' button to fire off the text to VRChat's OSC listener, triggering it to appear in the user's in-game chat box.

## Usage
Config file support will be added at some point to allow specifying model locations, OSC endpoint, etc.  Currently, just get the language model (see below), build, and run; or double-click the compiled binary.  Ensure OSC is enabled in VRChat or you won't have much luck sending any data to it.  Screenshots will be added eventually.

## Building
It's kind of a pain in the butt to be honest.  Full instructions can be found at [this page](https://github.com/tazz4843/whisper-rs/blob/master/BUILDING.md) in the whisper-rs Github repository.  The short version is:
* Install Visual C++ with CLANG enabled
* Install CMAKE
* Install CUDA if you have an Nvidia card
* Set LIBCLANG_PATH in git bash/powershell/etc.
* cargo build

NOTE: It seems to run like, way way better built with the default 'dev' profile as opposed to 'release'.  I do not know why this is the case.

NOTE 2: I have an AMD card so I can't use CUDA but if you have it just uncomment the appropriate line in Cargo.toml.

## Language Model
Whisper.cpp uses the OpenAI Whisper PyTorch models converted to a custom `ggml` format.  You can find more information about them at [the whisper.cpp Github](https://github.com/ggerganov/whisper.cpp/blob/master/models/README.md), or can just download pre-converted models from [the whisper.cpp HuggingFace repo](https://huggingface.co/ggerganov/whisper.cpp/tree/main).  MelonSTT is currently hard-coded to look for `ggml-tiny.en.bin` in the CWD it is run from; it's not the most thorough model but it's very small and only takes about a second to process recorded audio.

melonstt v0.35.1 23/Nov/2024