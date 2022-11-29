# Mira Screenshare Beta Testing

Thank you so much for your interest in trying out Mira.

## How to use

Download Mira [here](https://github.com/mira-screen-share/sharer/releases/tag/v0.1), and share the link to the viewers.

Use `-d` parameter to select the monitor you want to share if you have multiple (e.g. `-d 1` selects the second display). There is also a config file so you could tweak encoder settings or set your desired frame rate. Hardware encoding and VP9 codec are also experimentally supported.

* macOS users: you might need to install ffmpeg with `brew install ffmpeg`.
* Apple Silicon users: we don't yet have a native build and we have no idea if Rosetta would work.

## Intended use cases
Mira is intended to be used with people you trust. You could use it on your own device as a remote desktop,
or share your screen with your lab partner. It is not designed as an alternative for Google Meets, Zoom, etc.,
as it does not have webcam/voice capability. Although we do not enforce a maximal concurrent viewer limit,
we do not recommend to use it with more than 3 concurrent viewers.

## Security
Video and control events are transmitted directly between peers using [SRTP](https://en.wikipedia.org/wiki/Secure_Real-time_Transport_Protocol), once connection is established through our signalling server.
Anyone with the link could see your screen and control your mouse or keyboard (unless you disabled remote control with the param `--disable-control`)

## Known issues
This project is still in its early stage development phase.

* You might experience some higher than normal lag in macOS.
* The resolution is relatively low in macOS (Retina support is turned off currently)
* Latency is higher than normal (100-300ms)

## Survey

We would really appreciate it if you could fill out [this survey](https://5k3n24rfitw.typeform.com/to/puyonkFx) to give
us some feedback.
You could fill it out even if you havn't gotten a chance to use Mira yet.
