# MelonSTT
ilo tawa ni: ona li ante e toki kalama tawa toki sitelen.  ona li kepeken e ilo OSC, li pana tawa musi VRChat.

## sona tawa ni
ilo MelonSTT li ilo pona tawa ilo Windows.  ona li kepeken e sona sitelen pi toki Rustlang tan ilo Whisper.cpp.  kin la, ona li kepeken e ilo Slint.  ona o toki tawa ilo VRChat.  o kepeken e ilo OVR Toolkit anu ilo Desktop++.  jan kepeken li luka e leko, li toki kalama, li lukin e toki sitelen, li luke e leko ante.  toki sitelen li tawa e musi VRChat.

## kepeken
o alasa e ijo toki, kin o pali, kin o kepeken.  musi VRChat o ken e ilo OSC.

## pali
ni li ike lili.  o alasa e sona ale tan [lipu ni](https://github.com/tazz4843/whisper-rs/blob/master/BUILDING.md).  sona lili la, sina wile jo e ni:
* ilo Visual C++ en ilo CLANG
* ilo CMAKE
* sina jo ijo Nvidia la, ilo CUDA
* pana sona LIBCLANG_PATH insa selo sona (ilo 'git bash', ilo powershell)
* o pali e `cargo build`

SONA: o kepeken e kule 'dev' la, ilo ni li pali pona mute.  seme la, mi sona ala.

SONA 2: mi jo e ijo pi kulupu AMD pi sitelen lukin.  mi ken ala kepeken e ilo CUDA.  sina jo e ijo pi kulupu NVIDIA pi sitelen lukin la, sina ken ante e lipu Cargo.toml.

## ijo toki
ilo Whisper.cpp li kepeken e ijo pi ilo Whisper PyTorch pi kulupu OpenAI.  ona li lon e kule `ggml`.  o alasa e sona mute tan [ma Github tan ilo Whisper.cpp](https://github.com/ggerganov/whisper.cpp/blob/master/models/README.md).  anu, sina ken alasa e ijo lon [ma HuggingFace tan ilo Whisper.cpp](https://huggingface.co/ggerganov/whisper.cpp/tree/main).  ilo ni li kepeken e sitelen `ggml-tiny.en.bin` la, o pana e sitelen insa lon sama ilo ni.

SONA 3: sitelen tawa pi toki pona li lon insa ma Github ni.


ilo melonstt nanpa 0.35.1 tan kulupu Melondog Software