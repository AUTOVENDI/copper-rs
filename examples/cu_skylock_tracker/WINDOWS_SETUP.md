# SKYLOCK port: Windows machine setup

Sets up a Windows 11 x64 machine to build and run the port natively. Work through the sections in
order. "Admin PowerShell" means PowerShell opened with "Run as administrator". To edit your user
Path, search Start for "Edit environment variables for your account".

## 1. Before you start
- At least 100 GB free on an SSD, and admin rights.
- An NVIDIA GPU for `--features cuda`. Without one, skip section 4.
- Clone to `C:\src\copper-rs`, outside OneDrive. Desktop and Documents are often synced, and
  `target\` grows to tens of GB. A path without spaces also keeps the root justfile working.
- Turn on Developer Mode (Settings → System → For developers), so git can create the repo's three
  symlinks.
- Optional, for faster builds: put the clone on a Dev Drive (then use its path instead of
  `C:\src` below), or exclude the build folders from Defender in an admin PowerShell:
  `Add-MpPreference -ExclusionPath 'C:\src', "$env:USERPROFILE\.cargo", "$env:USERPROFILE\.rustup"`

## 2. Base tools
In an admin PowerShell:

```powershell
winget install --id Git.Git -e
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--passive --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
winget install --id Rustlang.Rustup -e
winget install --id Casey.Just -e
winget install --id NASM.NASM -e
winget install --id Python.PythonInstallManager -e
winget install --id Microsoft.VisualStudioCode -e   # skip if VS Code is installed
```

Then, in a new normal PowerShell:

```powershell
rustup default stable
rustup update
py install 3.13
nasm -v
```

- **Git for Windows** provides Git Bash. The `sh` behind `just` recipes runs in it.
- **Build Tools 2022**, not a newer release: nvcc rejects MSVC versions released after it, and
  CUDA 12.9 (section 4) predates VS 2026. If Visual Studio 2026 is already installed, keep it:
  Build Tools 2022 installs alongside it, and section 7's prompt selects 2022.
- **Rust:** the workspace needs 1.98 or newer.
- **NASM:** `aws-lc-sys`, the TLS library behind the model download, assembles with it. If
  `nasm -v` is not found, add the NASM folder (normally `C:\Program Files\NASM`) to your user Path.
- **Python:** the root justfile runs `python3` (3.11 or newer) every time it loads, so every
  `just` recipe needs it. The install manager provides a `python3` command. If python.org's
  installer already put Python 3.11 or newer on PATH, skip the install manager: copy that
  folder's `python.exe` to `python3.exe` (an all-users install needs an admin PowerShell). If
  `where.exe py` lists `C:\Windows\py.exe`, the old launcher comes before the install manager's
  `py`, so `py install` won't run.

## 3. GStreamer 1.28.9
1. Download `gstreamer-1.0-msvc-x86_64-1.28.9.exe` from
   <https://gstreamer.freedesktop.org/data/pkg/windows/1.28.9/msvc/>. Compare
   `Get-FileHash <file>` with the `.sha256sum` file next to it, then run it.
2. Choose an install type that includes the development files; a runtime-only install can't build
   the Rust bindings. Since 1.28 a single `.exe` (Inno Setup) holds both. Its default type,
   "Runtime and development headers", is the right one. GStreamer's install page and the
   gstreamer-rs README still describe two `.msi` files.
   - Install to a folder without spaces, such as `C:\gstreamer\1.0\msvc_x86_64`; the default is
     under `Program Files`. Unattended, from an admin PowerShell:
     `.\gstreamer-1.0-msvc-x86_64-1.28.9.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP- /ALLUSERS /TYPE=devel /DIR=C:\gstreamer\1.0\msvc_x86_64`
   - The installer sets `GSTREAMER_1_0_ROOT_MSVC_X86_64` but doesn't touch Path, and it ships
     its own `pkg-config.exe` in `bin`.
   - If the download crawls, try `curl.exe -4`; on 2026-11-20 IPv6 to the GStreamer CDN was
     10 times slower than IPv4.
3. In a new PowerShell, point pkg-config at it:

   ```powershell
   $gst = $env:GSTREAMER_1_0_ROOT_MSVC_X86_64
   if (-not $gst) { $gst = Read-Host 'GStreamer folder that contains bin and lib' }
   $gst = $gst.TrimEnd('\')
   [Environment]::SetEnvironmentVariable('PKG_CONFIG_PATH', "$gst\lib\pkgconfig", 'User')
   "Add $gst\bin to your user Path if a new terminal can't find gst-inspect-1.0."
   where.exe pkg-config
   ```

4. `pkg-config` comes from GStreamer's `bin`, once that folder is on your Path. Only if
   `where.exe` still finds none, run `winget install --id bloodrock.pkg-config-lite -e`.

## 4. CUDA (NVIDIA only)
1. Install the current NVIDIA driver.
2. Install CUDA Toolkit 12.9 from <https://developer.nvidia.com/cuda-toolkit-archive>. Choose
   Custom, and untick the driver if yours is newer. If the installer warns that it found no Visual
   Studio, carry on: nvcc uses the Build Tools compiler.
3. Why 12.9:
   - Candle 0.11 uses `cudarc` 0.19.10, which panics on any CUDA newer than 13.4. winget's default,
     13.5, therefore breaks the `cuda` build.
   - CUDA 13 also drops GPUs older than Turing. 12.9 covers Maxwell through Blackwell; RTX 50
     cards need 12.8 or newer.
4. nvcc needs `cl.exe` on PATH. The start routine in section 7 provides it.
5. Check that `C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9\bin` is on your Path. A
   silent install of selected subpackages sets `CUDA_PATH` but adds no Path entry.

## 5. Cargo tools
In a new PowerShell:

```powershell
cargo install --locked taplo-cli prek typos-cli
cargo install --locked fmtron --version 0.10.0
cargo install --locked rerun-cli --version 0.43.2
```

- `just fmt` runs taplo, fmtron, prek and typos. The justfile pins fmtron 0.10.0, though
  crates.io's latest is 1.0.0.
- The 0.43.2 viewer matches the workspace's `~0.43.2` Rerun pin, and the viz task's `spawn()`
  starts it. If a later build resolves a newer 0.43.x, install the viewer that matches. If Windows
  Firewall asks about `rerun.exe`, allowing Private networks is enough.
- These build from source; `rerun-cli` takes the longest.

## 6. Clone
In Git Bash:

```bash
git clone --config core.autocrlf=input --config core.symlinks=true \
  <this repository's URL> /c/src/copper-rs
```

- `core.autocrlf=input` keeps the working tree in LF, byte for byte what the repo holds, and
  turns any CRLF back into LF on commit. Without it, Git for Windows checks files out as CRLF.
  Git Bash 5.3 runs CRLF scripts correctly; how the formatters treat CRLF files is untested.
- To fix an existing clone instead of re-cloning (after turning on Developer Mode): in Git Bash,
  run `git config core.autocrlf input`, `git config core.symlinks true`,
  `git ls-files -z | xargs -0 touch -c`, `git checkout-index --force --all`,
  `git read-tree HEAD` and `git update-index -q --refresh`. Do it with nothing uncommitted.

## 7. Start a session
Start every session like this, so that VS Code and its terminals inherit `cl.exe` (for CUDA) and
the RTSP URL:
1. Quit VS Code completely.
2. From Start, open "x64 Native Tools Command Prompt for VS 2022". Check the full name: with
   Visual Studio 2026 installed, Start also lists "x64_x86 Cross Tools Command Prompt for VS",
   a 2026 prompt that builds 32-bit code with a compiler CUDA 12.9 rejects.
3. Paste this as one line, then paste the URL on its own at the `RTSP URL:` prompt:

   ```bat
   cd /d C:\src\copper-rs && set /p SKYLOCK_RTSP_URL=RTSP URL: && code .
   ```

`set /p` keeps the URL out of shell history. It takes the next line you type or paste, so
pasting several lines at once stores a command as the URL. Don't put the URL in a settings file,
which would keep it on disk.

## 8. Check
In VS Code's terminal (PowerShell), with VS Code started as in section 7:

```powershell
rustc --version              # 1.98 or newer
just --version
nasm -v
rerun --version              # 0.43.2
gst-inspect-1.0 --version    # 1.28.9
pkg-config --modversion gstreamer-1.0 gstreamer-app-1.0 gstreamer-video-1.0
foreach ($e in 'rtspsrc', 'rtph264depay', 'rtph265depay', 'h264parse', 'h265parse', 'avdec_h264', 'avdec_h265', 'videoconvert', 'videoscale', 'appsink', 'mfvideosrc') { gst-inspect-1.0 $e *> $null; if ($LASTEXITCODE -eq 0) { "ok       $e" } else { "MISSING  $e" } }
gst-launch-1.0 mfvideosrc ! videoconvert ! autovideosink   # webcam window; Ctrl+C to stop
# NVIDIA only:
nvidia-smi --query-gpu=name,compute_cap,driver_version --format=csv
nvcc --version               # 12.9
where.exe cl                 # first hit under 2022\BuildTools\...\Hostx64\x64
```

In Git Bash (in VS Code's terminal, pick Git Bash):

```bash
python3 -c 'import sys, tomllib; print(sys.version)'   # 3.11 or newer
cd /c/src/copper-rs && just --list | head -5           # loads the root justfile
case "$SKYLOCK_RTSP_URL" in rtsp*://*) echo "RTSP URL set" ;; "") echo "RTSP URL missing" ;; *) echo "RTSP URL is not rtsp://" ;; esac
```

- If `python3` says "Python was not found", turn off the App Installer `python.exe` and
  `python3.exe` entries under Settings → Apps → Advanced app settings → App execution aliases,
  then reopen the terminal.
- Run `just` from Git Bash, because PowerShell has no `sh` on PATH.

RTSP stream: these print the URL, so run them where nobody else reads the output.

```powershell
gst-discoverer-1.0 $env:SKYLOCK_RTSP_URL             # codec, resolution, frame rate
gst-launch-1.0 playbin "uri=$env:SKYLOCK_RTSP_URL"   # live view; Ctrl+C to stop
```

Then run the app as `README.md` describes.
