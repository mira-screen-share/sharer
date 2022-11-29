# Mira Screenshare Beta Testing

Thank you so much for your interest in trying out Mira.

## Survey

We would really appreciate it if you could fill out [this survey](https://5k3n24rfitw.typeform.com/to/puyonkFx) to give
us some feedback.

You can fill it out even if you havn't gotten a chance/don't want to use Mira yet, but you're more than welcome to give it a try following the instructions below:

## How to use

1. Download the Mira sharer executable for your OS [here](https://github.com/mira-screen-share/sharer/releases/tag/v0.1).

> - Apple Silicon users: we don't yet have a native build and we have no idea if Rosetta would work.

2. 
- macOS with Intel CPU:
  1. Install ffmpeg with `brew install ffmpeg`.
  2. Give the executable permission with `chmod +x mira_sharer_macos_intel`.
  3. Attempt to launch it with `./mira_sharer_macos_intel`.
  4. You might get 
      > "mira_sharer_macos_intel” cannot be opened because the developer cannot be verified.
     
     If so, please navigate to System Preferences -> Security & Privacy -> General and click "Allow Anyway" at the bottom for "mira_sharer_macos_intel", then try step iii again.
 
- Windows:
  1. Unzip `mira_sharer_windows_x64.zip`
  2. Run `mira_sharer.exe`

3. [Optional] Use `-d` parameter to select the monitor you want to share if you have multiple (e.g. `-d 1` selects the second display). There is also a config file so you could tweak encoder settings or set your desired frame rate. Hardware encoding and VP9 codec are also experimentally supported.
4. Share the "Invite link" (found in the second line of the INFO log) to other people, they can simply open it in a browser to start viewing & controlling your screen.

## Intended use cases
Mira is intended to be used with people you trust. You could use it on your own device as a remote desktop,
or share your screen with your lab partner. It is not designed as an alternative for Google Meets, Zoom, etc.,
as it does not have webcam/voice capability. Although we do not enforce a maximal concurrent viewer limit,
we do not recommend to use it with more than 3 concurrent viewers.

## Security
Video and control events are transmitted directly between peers using [SRTP](https://en.wikipedia.org/wiki/Secure_Real-time_Transport_Protocol), once connection is established through our signalling server.
Anyone with the link could see your screen and control your mouse or keyboard (unless you disabled remote control with the param `--disable-control`)

## Known issues
This project is still in its early stage development phase, further optimization is anticipated, especially for macOS.

* Latency might (frequently) be higher than what we're aiming for (<100ms).
* The resolution is relatively low in macOS (capturing at full Retina resolution is turned off currently) since the encoding performance can't keep up.
