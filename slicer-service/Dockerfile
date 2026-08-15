# Bambu Studio's CLI is Qt/GTK/WebKit-linked even for headless use — the full runtime lib list
# below was found empirically (iterate: run --help, add whatever .so it's missing, repeat) since
# BambuStudio ships no documented list of them. ubuntu24.04 matches the AppImage build fetched
# below, avoiding glibc/lib version mismatches a different base could introduce.
FROM ubuntu:24.04

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl ca-certificates file \
    squashfs-tools \
    xvfb \
    python3 python3-venv python3-pip \
    libfontconfig1 libgl1 libglu1-mesa libxkbcommon0 libxkbcommon-x11-0 \
    libxcb-cursor0 libxcb-icccm4 libxcb-image0 libxcb-keysyms1 libxcb-randr0 \
    libxcb-render-util0 libxcb-shape0 libxcb-xinerama0 libxcb-xkb1 \
    libdbus-1-3 libsm6 libice6 libgstreamer1.0-0 libgstreamer-plugins-base1.0-0 \
    libglib2.0-0 libnss3 libsecret-1-0 libgtk-3-0 libwebkit2gtk-4.1-0 libsoup-3.0-0 \
    && rm -rf /var/lib/apt/lists/*

# Latest stable, fetched at build time — not pinned. CI rebuilds and re-runs the slicer smoke
# test (tests/test_slice.py) against whatever's current before pushing the image, so a slicer
# regression fails the build rather than shipping silently.
#
# Extracted with unsquashfs rather than the AppImage's own --appimage-extract: an AppImage is
# just an ELF stub + an appended squashfs, found here via its magic bytes ("hsqs") rather than
# executing the AppImage's own runtime, which needs FUSE or the exact right libc on whatever's
# building this, neither guaranteed in every CI/build environment. (7z can read the same
# squashfs but Ubuntu's p7zip-full build lacks zstd codec support — E_NOTIMPL — which is what
# BambuStudio's AppImage uses; unsquashfs has zstd built in.)
RUN LATEST_URL=$(curl -fsSL https://api.github.com/repos/bambulab/BambuStudio/releases/latest \
      | grep -o '"browser_download_url": *"[^"]*ubuntu24\.04[^"]*\.AppImage"' \
      | head -1 | cut -d'"' -f4) \
    && curl -fSL --retry 5 --retry-delay 5 --retry-connrefused -o /tmp/bambu.AppImage "$LATEST_URL" \
    && OFFSET=$(python3 -c "print(open('/tmp/bambu.AppImage','rb').read().rfind(b'hsqs'))") \
    && unsquashfs -o "$OFFSET" -d /opt/bambustudio /tmp/bambu.AppImage \
    && chmod +x /opt/bambustudio/bin/bambu-studio \
    && rm /tmp/bambu.AppImage

# Mirrors the AppImage's own AppRun script (LD_LIBRARY_PATH + the LC_ALL=C segfault
# workaround BambuStudio's own AppRun carries) — bambu-studio on PATH either way.
RUN printf '#!/bin/bash\nexport LD_LIBRARY_PATH=/opt/bambustudio/bin\nexport LC_ALL=C\nexec /opt/bambustudio/bin/bambu-studio "$@"\n' \
    > /usr/local/bin/bambu-studio && chmod +x /usr/local/bin/bambu-studio

WORKDIR /app
COPY requirements.txt .
RUN python3 -m venv .venv && .venv/bin/pip install --no-cache-dir -r requirements.txt
COPY main.py .

ENV PATH="/app/.venv/bin:${PATH}"
EXPOSE 8100
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8100"]
