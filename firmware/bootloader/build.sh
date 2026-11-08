#!/bin/sh
# Builds firmware/bootloader/bootloader.bin (0086) inside Espressif's pinned
# ESP-IDF image under rootless Docker; nothing is installed on the host.
# Rebuilding is a toolchain change: ask first, then record the new hashes.
set -eu
IMAGE="${IDF_IMAGE:-espressif/idf:v6.1}"
here="$(cd "$(dirname "$0")" && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/main"
cp "$here/sdkconfig.defaults" "$work/"
cat > "$work/CMakeLists.txt" <<'CM'
cmake_minimum_required(VERSION 3.16)
include($ENV{IDF_PATH}/tools/cmake/project.cmake)
project(overhead_bootloader)
CM
printf 'idf_component_register(SRCS "main.c")\n' > "$work/main/CMakeLists.txt"
printf 'void app_main(void) {}\n' > "$work/main/main.c"
docker run --rm -v "$work:/project" -w /project "$IMAGE" sh -c '
  set -e
  idf.py set-target esp32s3 >/dev/null
  idf.py bootloader >/dev/null
  git -C "$IDF_PATH" rev-parse HEAD > idf-commit.txt
  git -C "$IDF_PATH" describe --tags --always >> idf-commit.txt
  chmod -R a+rw /project
'
cp "$work/build/bootloader/bootloader.bin" "$here/bootloader.bin"
cp "$work/sdkconfig" "$here/sdkconfig"
cp "$work/idf-commit.txt" "$here/idf-commit.txt"
docker image inspect --format '{{index .RepoDigests 0}}' "$IMAGE" > "$here/image-digest.txt"
(cd "$here" && sha256sum bootloader.bin > bootloader.bin.sha256)
cat "$here/idf-commit.txt" "$here/image-digest.txt" "$here/bootloader.bin.sha256"
