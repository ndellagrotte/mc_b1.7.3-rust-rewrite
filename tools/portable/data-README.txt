This data folder is the game's data directory: it sits next to the game
(the .AppImage, mc_b173.exe or mc_b173.app), so the game keeps everything here.
options.txt appears after the first run; worlds are in saves/, F2 screenshots
in screenshots/, statistics in stats/, world exports in exports/.

Upgrading: there is no auto-update. Unzip the new build, then copy this data
folder over the new build's data folder.
--data-dir DIR overrides this folder. Delete or rename it and the game uses
the normal platform data folder instead.
The game still needs a Prism Launcher b1.7.3 instance for minecraft.jar and
resources/: import b1.7.3-mmc-instance.zip (next to this folder) into Prism
Launcher (Add Instance > Import) and launch it once (see RUNNING.md).

Linux: needs glibc 2.31 or newer and the system's ALSA, libudev, Wayland or
X11 and GPU driver. Without FUSE, run ./mc_b173.AppImage --appimage-extract-and-run.
If your unzip tool dropped the executable bit: chmod +x mc_b173.AppImage.
Windows: the exe is unsigned; at the SmartScreen warning choose "More info",
then "Run anyway".
macOS: run xattr -dr com.apple.quarantine on the unzipped folder first, or
macOS runs the app from a hidden copy and does not see this folder.
