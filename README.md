# MelonSTT
Speech-to-Text to VRChat via OSC

## About
MelonSTT is a simple speech-to-text transcriber program for Windows utilizing the whisper-rs Rust bindings to whisper.cpp for the transcription, and slint for the front-end.  It is specifically intended to be used with VRChat via some manner of in-game desktop interface (OVR Toolkit, Desktop++, etc) - the user simply clicks on one of the record buttons, speaks into their microphone, reviews the transcribed text, and clicks the 'send' button to fire off the text to VRChat's OSC listener, triggering it to appear in the user's in-game chat box.

## Usage
Once compiled, the program expects to find `melon.toml` in `$CWD`.  You can edit this file to specify the particular location of the language model you want to use, and if VRChat is listening for OSC on a non-standard port, that can be specified as well.  There IS a language field, but it currently doesn't do anything; eventually it'll allow specifying what language the model is in (and ostensibly what language the speaker will be using).

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
Whisper.cpp uses the OpenAI Whisper PyTorch models converted to a custom `ggml` format.  You can find more information about them at [the whisper.cpp Github](https://github.com/ggerganov/whisper.cpp/blob/master/models/README.md), or can just download pre-converted models from [the whisper.cpp HuggingFace repo](https://huggingface.co/ggerganov/whisper.cpp/tree/main).  Testing has all been done using `ggml-tiny.en.bin` (and to a lesser extent, a converted toki pona language model); it's not the most thorough, but it's small and runs reasonably fast.  Obviously it'd run a hell of a lot faster with hardware acceleration (read: CUDA) but whatever.

melonstt v0.43.11 23/Nov/2024