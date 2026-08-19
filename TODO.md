# TODO

- [x] Fix images on powershell (Needs testing)
- [x] Fix Displays on windows
- [x] Make GPU Drivers look for the default drivers properly on Windows (Needs testing)
- [ ] Improve overall performance on windows (Needs testing)
- [x] Improve disk storage customization
- [x] Update load Order!!
- [x] Improve file structure on windows

## Needs a real Windows box

Everything below builds for x86_64-pc-windows-gnu and runs under wine, which
is not the same as being right.

- [ ] Displays: wine reports both monitors correctly through
      EnumDisplayDevicesW/EnumDisplaySettingsW. Worth checking a mixed-refresh
      or scaled setup, and one with a monitor attached but asleep.
- [ ] GPU: the driver key now comes from the adapter drawing the desktop
      instead of Class\{4d36e968-…}\0000. The case that motivated it — a
      laptop with an iGPU and a dGPU — can't be reproduced here, so it still
      wants a hybrid machine.
- [ ] Packages and disks read as pacman/0.00 GiB under wine (host leaking
      through). Confirm choco/winget detection on the real thing.

## Weird quirk on windows

When loading images, it "freezes" and only finishes when the user cancel by himself.

Probably fixed, unverified. Two things caused hangs and both are gone:
viuer's kitty probe blocks on a terminal reply that Windows Terminal and
PowerShell may never send, and the old GIF path slept through every frame in
process. The fallback now passes `use_kitty: false` so viuer never runs that
probe, and the kitty path detects from the environment instead of asking.
Still wants a real Windows box to confirm.

## Notes

- `nofetch --color` is `-C`; `-c` is `--config`. `--no-animation` is `-A`,
  with `--no-anim` as a hidden alias.
- Kitty graphics are handled natively in `src/render/kitty.rs`; viuer only
  covers sixel, iTerm and half-blocks now. Detection is environment-based, so
  `NOFETCH_KITTY=0|1|full` overrides it when a terminal is misidentified.
- GIF art picks whichever protocol lets the *terminal* animate, since that is
  the only kind nofetch can exit out from under:
    - kitty                        -> kitty `a=f` frames + `a=a` playback
    - WezTerm/iTerm2/mintty/rio/
      Warp/Konsole                 -> iTerm2 OSC 1337 with the raw GIF bytes,
                                      via viuer (this is what the pre-rewrite
                                      code was accidentally relying on)
    - Ghostty, and anything else   -> nothing native exists, so nofetch drives
                                      the frames itself and blocks until Ctrl-C
  Still images always take the kitty path where it exists; only GIFs route.
- `--no-animation` cuts every branch of that above, not just the blocking one:
  the iTerm2 handoff is skipped (raw GIF bytes animate themselves there), the
  kitty path draws the root frame as a still and uploads no frames, and the
  viuer fallback decodes frame one and prints *that* rather than the file. It
  is a no-op for ASCII art and for still images.
- The client-driven path uploads each frame once (`a=t`) and then flips with a
  delete + placement pair, ~94 bytes a frame. The old loop re-transmitted the
  whole frame every time, ~220 KB a frame, forever.
- `default_art.txt` is compiled in with `include_str!`, so it has to stay in
  the repo root for the build to work.
