#!/usr/bin/env bash
# Drive a board plugged into the user's Mac from devbox, over SSH (host alias
# `mac` in ~/.ssh/config). Builds stay on devbox; only flashing and UART
# capture run on the Mac, through the forced-command gate
# ~/code/devbox/mac/overhead-gate (the key can do nothing else there).
# Logs stream back to devbox files.
#
#   ./mac.sh info                  port, chip and MAC check, then hard reset
#   ./mac.sh read SECS LOG         read the running board without reset
#   ./mac.sh monitor SECS LOG      reset, then capture SECS seconds
#   ./mac.sh flash ELF LOG [SECS]  USB flash (bootloader, table, erase otadata,
#                                  app in ota_0; settings/elements kept), then
#                                  monitor SECS seconds after flashing (default 120)
#   ./mac.sh erase MAC             erase the whole chip (new/gift boards): the
#                                  board's MAC must be given and must match
#
# Env: OVERHEAD_MAC_HOST (default mac), OVERHEAD_MAC_PORT (default: the single
# /dev/cu.usbserial-* on the Mac), OVERHEAD_MAC (expected board MAC, default
# the user's board; flash refuses on mismatch).
set -euo pipefail

HOST="${OVERHEAD_MAC_HOST:-mac}"
EXPECT_MAC="${OVERHEAD_MAC:-80:45:6b:2a:45:08}"
HERE="$(cd "$(dirname "$0")" && pwd)"

die() { echo "mac.sh: $*" >&2; exit 1; }
gate() { ssh "$HOST" "$*"; }

port() {
    if [[ -n "${OVERHEAD_MAC_PORT:-}" ]]; then echo "$OVERHEAD_MAC_PORT"; return; fi
    local ports
    ports="$(gate ports)"
    [[ -n "$ports" ]] || die "no /dev/cu.usbserial-* on $HOST (board unplugged?)"
    [[ "$(wc -l <<<"$ports")" -eq 1 ]] || die "several ports on $HOST, set OVERHEAD_MAC_PORT: $ports"
    echo "/dev/$ports"
}

# board-info enters download mode; the gate hard-resets afterwards.
check_mac() {
    local info mac
    info="$(gate board-info "$1" 2>&1)" || { echo "$info" >&2; die "board-info failed on $1"; }
    mac="$(grep -oE 'MAC address: +[0-9a-f:]+' <<<"$info" | awk '{print $3}')"
    grep -E 'Chip type|Flash size|MAC address' <<<"$info"
    [[ "$mac" == "$EXPECT_MAC" ]] || die "board MAC $mac != expected $EXPECT_MAC (set OVERHEAD_MAC to override)"
}

put() {
    gate put "$2" < "$1"
    local want got
    want="$(sha256sum "$1" | cut -d' ' -f1)"
    got="$(gate sha256 "$2" | cut -d' ' -f1)"
    [[ "$want" == "$got" ]] || die "$2 hash mismatch after upload ($want vs $got)"
}

new_log() { [[ ! -e "$1" ]] || die "refusing to overwrite $1"; mkdir -p "$(dirname "$1")"; }

cmd="${1:-}"; shift || true
case "$cmd" in
info)
    p="$(port)"; echo "port $p"; check_mac "$p" ;;
read)
    [[ $# -eq 2 ]] || die "usage: read SECS LOG"
    new_log "$2"; p="$(port)"
    gate read "$p" "$1" > "$2" || true
    echo "wrote $2 ($(wc -c <"$2") bytes)" ;;
monitor)
    [[ $# -eq 2 ]] || die "usage: monitor SECS LOG"
    new_log "$2"; p="$(port)"
    gate monitor "$p" "$1" > "$2" 2>&1 || true
    echo "wrote $2 ($(wc -c <"$2") bytes)" ;;
flash)
    [[ $# -ge 2 ]] || die "usage: flash ELF LOG [SECS]"
    elf="$1" log="$2" secs="${3:-120}"
    [[ -f "$elf" ]] || die "no ELF $elf"
    new_log "$log"; p="$(port)"; echo "port $p"
    check_mac "$p"
    put "$elf" app.elf
    put "$HERE/bootloader/bootloader.bin" bootloader.bin
    put "$HERE/partitions.csv" partitions.csv
    echo "ELF sha256 $(sha256sum "$elf" | cut -d' ' -f1)"
    # Upper bound covers the write; stop SECS after the flash itself finishes.
    gate flash "$p" "$((secs + 480))" > "$log" 2>&1 &
    flasher=$!
    until grep -qaE 'Flashing has completed|Error' "$log" 2>/dev/null; do
        kill -0 "$flasher" 2>/dev/null || break; sleep 2
    done
    if grep -qa 'Flashing has completed' "$log"; then sleep "$secs"; fi
    gate stop || true
    wait "$flasher" || true
    grep -qa 'Flashing has completed' "$log" || die "flash did not complete; see $log"
    echo "wrote $log ($(wc -c <"$log") bytes)" ;;
erase)
    [[ $# -eq 1 ]] || die "usage: erase MAC (the board's, from ./mac.sh info)"
    [[ "$1" =~ ^([0-9a-f]{2}:){5}[0-9a-f]{2}$ ]] || die "not a MAC: $1"
    p="$(port)"; echo "port $p"
    EXPECT_MAC="$1" check_mac "$p"
    echo "erasing $1 (settings, elements and app)"
    gate erase "$p" ;;
*)
    sed -n '2,19p' "$0" | sed 's/^# \{0,1\}//'; exit 2 ;;
esac
