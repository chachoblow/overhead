//! 0087 setup-window rules, host-tested: the boot-hold gesture, when the
//! window opens, the scan list sent to the app, how a `Configure` becomes
//! settings (0084), and when setup gives up and runs on what is stored. The
//! target side (`setup_mode`) owns the radio, BLE and panel.
use crate::settings_store::{Settings, Site, Wifi};
use overhead_setup::{Configure, ConfigureResult, Security, SetupReason};

/// Held this long at boot: setup.
pub const SETUP_HOLD_MS: u64 = 3_000;
/// Held this long: factory reset.
pub const RESET_HOLD_MS: u64 = 10_000;
/// The stored network must join within this long after boot, or the device
/// restarts into setup.
pub const JOIN_TIMEOUT_S: u64 = 5 * 60;
/// With settings stored, setup with no app connected for this long restarts
/// into the normal run (a router slower than the device after a power cut
/// must not strand it in setup).
pub const SETUP_IDLE_S: u64 = 10 * 60;
/// Networks sent per scan, strongest first.
pub const SCAN_MAX: usize = 20;
/// While no app is connected the device rescans this often, so a `Scan` is
/// answered without sweeping channels under a live BLE link (a sweep can
/// outlast a central's supervision timeout).
pub const RESCAN_S: u64 = 60;
/// A cached scan younger than this answers `Scan`; older, the device scans.
pub const SCAN_CACHE_S: u64 = 120;

/// What the knob switch held at boot asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootHold {
    None,
    Setup,
    FactoryReset,
}

pub fn classify_hold(held_ms: u64) -> BootHold {
    match held_ms {
        ms if ms >= RESET_HOLD_MS => BootHold::FactoryReset,
        ms if ms >= SETUP_HOLD_MS => BootHold::Setup,
        _ => BootHold::None,
    }
}

/// Whether to open the setup window, and why. `join_timed_out`: the previous
/// boot gave up joining (a flag kept across the reset). On probation (0086)
/// the gestures and the join flag wait: the health check decides first, and
/// a stored record always exists there.
pub fn setup_reason(
    settings: &Settings,
    hold: BootHold,
    join_timed_out: bool,
    on_probation: bool,
) -> Option<SetupReason> {
    if settings.wifi.is_none() || settings.site.is_none() {
        return Some(SetupReason::Unconfigured);
    }
    if on_probation {
        return None;
    }
    match hold {
        BootHold::FactoryReset => Some(SetupReason::FactoryReset),
        BootHold::Setup => Some(SetupReason::Button),
        BootHold::None if join_timed_out => Some(SetupReason::JoinTimeout),
        BootHold::None => None,
    }
}

/// What a factory reset keeps: nothing personal. The channel returns to
/// `stable`; the failed build is kept so a known-bad image is not offered
/// again.
pub fn factory_reset(s: &Settings) -> Settings {
    Settings {
        failed_build: s.failed_build,
        ..Settings::defaults(overhead_update::Channel::Stable)
    }
}

/// Checks a `Configure` with the settings record's own rules and returns
/// the network and site to test.
pub fn accept(c: &Configure) -> Result<(Wifi, Site), ConfigureResult> {
    c.check()?;
    let wifi = Wifi::new(c.ssid, c.passphrase).ok_or(ConfigureResult::InvalidPassphrase)?;
    let site = Site {
        lat_deg: c.lat_deg,
        lon_deg: c.lon_deg,
        alt_m: c.alt_m,
    };
    if !site.valid() {
        return Err(ConfigureResult::InvalidSite);
    }
    Ok((wifi, site))
}

/// The record to save after a successful test: the new network and site,
/// everything else (channel, failed/pending build) kept.
pub fn configured(s: &Settings, wifi: Wifi, site: Site) -> Settings {
    Settings {
        wifi: Some(wifi),
        site: Some(site),
        ..*s
    }
}

/// Up to `SCAN_MAX` networks, one entry per SSID (its strongest access
/// point and that point's channel), strongest first; hidden (empty) SSIDs
/// are left out.
#[derive(Clone)]
pub struct NetworkList {
    /// SSID, RSSI (dBm), security, channel.
    entries: [(Ssid, i8, Security, u8); SCAN_MAX],
    len: usize,
}

#[derive(Clone, Copy)]
struct Ssid {
    bytes: [u8; overhead_setup::SSID_MAX],
    len: u8,
}

impl Ssid {
    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or("")
    }
}

impl Default for NetworkList {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkList {
    pub const fn new() -> Self {
        Self {
            entries: [(
                Ssid {
                    bytes: [0; overhead_setup::SSID_MAX],
                    len: 0,
                },
                0,
                Security::Unsupported,
                0,
            ); SCAN_MAX],
            len: 0,
        }
    }

    /// The channel of `ssid`'s strongest access point, if it was seen.
    pub fn channel_of(&self, ssid: &str) -> Option<u8> {
        self.entries[..self.len]
            .iter()
            .find(|e| e.0.as_str() == ssid)
            .map(|e| e.3)
            .filter(|c| (1..=14).contains(c))
    }

    pub fn offer(&mut self, ssid: &str, rssi_dbm: i8, security: Security, channel: u8) {
        let b = ssid.as_bytes();
        if b.is_empty() || b.len() > overhead_setup::SSID_MAX || b.iter().all(|c| *c == 0) {
            return;
        }
        if let Some(e) = self.entries[..self.len]
            .iter_mut()
            .find(|e| e.0.as_str() == ssid)
        {
            if rssi_dbm > e.1 {
                e.1 = rssi_dbm;
                e.2 = security;
                e.3 = channel;
            }
            return;
        }
        let mut s = Ssid {
            bytes: [0; overhead_setup::SSID_MAX],
            len: b.len() as u8,
        };
        s.bytes[..b.len()].copy_from_slice(b);
        if self.len < SCAN_MAX {
            self.entries[self.len] = (s, rssi_dbm, security, channel);
            self.len += 1;
        } else if let Some(weakest) = self.entries.iter_mut().min_by_key(|e| e.1)
            && rssi_dbm > weakest.1
        {
            *weakest = (s, rssi_dbm, security, channel);
        }
    }

    /// Strongest first (ties keep scan order).
    pub fn sorted(&mut self) -> impl Iterator<Item = overhead_setup::Network<'_>> {
        let entries = &mut self.entries[..self.len];
        // Insertion sort: at most 20 entries, no allocation.
        for i in 1..entries.len() {
            let mut j = i;
            while j > 0 && entries[j - 1].1 < entries[j].1 {
                entries.swap(j - 1, j);
                j -= 1;
            }
        }
        entries
            .iter()
            .map(|(s, rssi, sec, _)| overhead_setup::Network {
                rssi_dbm: *rssi,
                security: *sec,
                ssid: s.as_str(),
            })
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// The advertised name, `OVERHEAD-` and the last two MAC bytes in hex.
pub fn ble_name(mac: [u8; 6]) -> [u8; 13] {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut name = *b"OVERHEAD-0000";
    for (i, b) in [mac[4], mac[5]].iter().enumerate() {
        name[9 + 2 * i] = HEX[usize::from(b >> 4)];
        name[10 + 2 * i] = HEX[usize::from(b & 0xf)];
    }
    name
}

/// Short phrase for the Sharp after a failed test.
pub fn failure_phrase(r: ConfigureResult) -> &'static str {
    match r {
        ConfigureResult::Saved => "saved",
        ConfigureResult::InvalidSsid => "name not valid",
        ConfigureResult::InvalidPassphrase => "password not valid",
        ConfigureResult::InvalidSite => "location not valid",
        ConfigureResult::NetworkNotFound => "network not found",
        ConfigureResult::WrongPassphrase => "wrong password",
        ConfigureResult::NoAddress => "no address from the router",
        ConfigureResult::JoinFailed => "could not join",
        ConfigureResult::SaveFailed => "could not save",
    }
}

/// The reset-surviving flag `setup_mode` keeps in RTC memory: a magic word
/// so power-on garbage never reads as a request.
pub const JOIN_TIMEOUT_FLAG: u32 = 0x4F48_4A54; // "OHJT"

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use overhead_update::Channel;

    fn stored() -> Settings {
        Settings {
            channel: Channel::Beta,
            failed_build: Some(9),
            pending_build: None,
            wifi: Wifi::new("home", "password1"),
            site: Some(Site {
                lat_deg: 51.5,
                lon_deg: -0.1,
                alt_m: 20.0,
            }),
        }
    }

    #[test]
    fn holds_classify_at_the_thresholds() {
        assert_eq!(classify_hold(0), BootHold::None);
        assert_eq!(classify_hold(2_999), BootHold::None);
        assert_eq!(classify_hold(3_000), BootHold::Setup);
        assert_eq!(classify_hold(9_999), BootHold::Setup);
        assert_eq!(classify_hold(10_000), BootHold::FactoryReset);
        assert_eq!(classify_hold(60_000), BootHold::FactoryReset);
    }

    #[test]
    fn the_window_opens_for_each_reason() {
        let s = stored();
        let empty = Settings::defaults(Channel::Stable);
        let no_site = Settings { site: None, ..s };
        use SetupReason::*;
        assert_eq!(
            setup_reason(&empty, BootHold::None, false, false),
            Some(Unconfigured)
        );
        assert_eq!(
            setup_reason(&no_site, BootHold::None, false, false),
            Some(Unconfigured)
        );
        assert_eq!(setup_reason(&s, BootHold::None, false, false), None);
        assert_eq!(
            setup_reason(&s, BootHold::Setup, false, false),
            Some(Button)
        );
        assert_eq!(
            setup_reason(&s, BootHold::FactoryReset, true, false),
            Some(FactoryReset)
        );
        assert_eq!(
            setup_reason(&s, BootHold::None, true, false),
            Some(JoinTimeout)
        );
        // Probation: only a missing record opens it.
        assert_eq!(setup_reason(&s, BootHold::Setup, true, true), None);
        assert_eq!(setup_reason(&s, BootHold::FactoryReset, false, true), None);
        assert_eq!(
            setup_reason(&empty, BootHold::None, false, true),
            Some(Unconfigured)
        );
    }

    #[test]
    fn factory_reset_keeps_only_the_failed_build() {
        let r = factory_reset(&stored());
        assert_eq!(r.wifi, None);
        assert_eq!(r.site, None);
        assert_eq!(r.channel, Channel::Stable);
        assert_eq!(r.failed_build, Some(9));
        assert_eq!(r.pending_build, None);
    }

    #[test]
    fn configure_is_checked_and_keeps_other_settings() {
        let c = Configure {
            ssid: "new",
            passphrase: "correct horse",
            lat_deg: 40.0,
            lon_deg: -74.0,
            alt_m: 10.0,
        };
        let (wifi, site) = accept(&c).unwrap();
        let s = configured(&stored(), wifi, site);
        assert_eq!(s.wifi.unwrap().ssid(), "new");
        assert_eq!(s.wifi.unwrap().passphrase(), "correct horse");
        assert_eq!(s.site.unwrap().lat_deg, 40.0);
        assert_eq!(s.channel, Channel::Beta);
        assert_eq!(s.failed_build, Some(9));
        // The protocol's rules and the record's agree.
        for (ssid, pass, lat, want) in [
            ("", "password1", 0.0, ConfigureResult::InvalidSsid),
            ("x", "short", 0.0, ConfigureResult::InvalidPassphrase),
            ("x", "password1", 91.0, ConfigureResult::InvalidSite),
        ] {
            let c = Configure {
                ssid,
                passphrase: pass,
                lat_deg: lat,
                ..c
            };
            assert_eq!(accept(&c).map(|_| ()), Err(want));
        }
    }

    #[test]
    fn scan_list_dedupes_sorts_and_caps() {
        let mut l = NetworkList::new();
        l.offer("", -30, Security::Open, 1);
        l.offer("\0\0", -30, Security::Open, 1);
        l.offer("b", -70, Security::Personal, 1);
        l.offer("a", -50, Security::Personal, 6);
        l.offer("b", -40, Security::Unsupported, 11); // stronger AP of the same SSID
        l.offer("a", -90, Security::Open, 3); // weaker: ignored
        assert_eq!(l.channel_of("a"), Some(6));
        assert_eq!(l.channel_of("b"), Some(11));
        assert_eq!(l.channel_of("c"), None);
        let got: std::vec::Vec<_> = l
            .sorted()
            .map(|n| (n.ssid, n.rssi_dbm, n.security))
            .collect();
        assert_eq!(
            got,
            [
                ("b", -40, Security::Unsupported),
                ("a", -50, Security::Personal)
            ]
        );

        let mut l = NetworkList::new();
        for i in 0..30i8 {
            let name = std::format!("n{i}");
            l.offer(&name, -100 + i, Security::Personal, 0);
        }
        assert_eq!(l.len(), SCAN_MAX);
        let got: std::vec::Vec<_> = l.sorted().map(|n| n.rssi_dbm).collect();
        assert_eq!(got.first(), Some(&-71));
        assert_eq!(got.last(), Some(&-90));
        assert!(got.windows(2).all(|w| w[0] >= w[1]));
    }

    #[test]
    fn names_and_phrases() {
        assert_eq!(
            &ble_name([0x80, 0x45, 0x6b, 0x2a, 0x45, 0x08]),
            b"OVERHEAD-4508"
        );
        assert_eq!(
            failure_phrase(ConfigureResult::WrongPassphrase),
            "wrong password"
        );
    }
}
