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

## Notes

- `nofetch --color` is `-C`; `-c` is `--config`.
- `default_art.txt` is compiled in with `include_str!`, so it has to stay in
  the repo root for the build to work.
