//! Host-only composition of the proposed live-UTC ISS radar path. The fixture
//! supplies *elements*, never a clock. No target loop, socket, COM-edge proof
//! or service guarantee while an actual blocking operation is in progress.
use crate::{
    coast_measurement::{Screen, app as fixture_app},
    device_timing::{DiagnosticAction, LifecycleAction},
    live_utc,
    sharp_lines::{Kind, LineSender},
    sntp_screen::{self, SntpOutcome, SntpRequest},
    utc_trust::{ObservationOutcome, UtcTrustPreflight},
};
use overhead_app::RadarApp;
use overhead_core::sgp4::chrono::{DateTime, NaiveDateTime};
use overhead_render::{CoastLayer, Layout, PeriodCache, draw_cached};

const NONCE: u64 = 0x1234_5678_90ab_cdef;
const NTP_OFFSET: i64 = 2_208_988_800;

fn ntp(unix_us: i64) -> u64 {
    let sec = (unix_us.div_euclid(1_000_000) + NTP_OFFSET) as u64;
    let frac = ((unix_us.rem_euclid(1_000_000) as u64) << 32).div_ceil(1_000_000);
    (sec << 32) | frac
}

fn reply(utc_us: i64, li: u8, stratum: u8) -> [u8; 48] {
    let mut bytes = [0; 48];
    bytes[0] = (li << 6) | (4 << 3) | 4; // server, version 4
    bytes[1] = stratum;
    bytes[24..32].copy_from_slice(&NONCE.to_be_bytes());
    bytes[32..40].copy_from_slice(&ntp(utc_us).to_be_bytes());
    bytes[40..48].copy_from_slice(&ntp(utc_us + 1_000).to_be_bytes());
    bytes
}

// The future socket adapter must first verify the exact peer IP/port and
// supply bracketing captures; this test exercises its accepted-packet boundary.
fn screened(bytes: &[u8], sent: u64, received: u64) -> SntpOutcome {
    let mut pending = Some(SntpRequest {
        transmit: NONCE,
        sent_uptime_us: sent,
    });
    let c = sntp_screen::bracketing_capture_bound_us(received - sent, 20).unwrap();
    let result = sntp_screen::screen(&mut pending, bytes, received, c);
    if matches!(result, SntpOutcome::Observation(_)) {
        assert!(pending.is_none());
    }
    result
}

// Only a screened observation may anchor; unmatched, bad-quality and KoD
// cannot renew. A real adapter must separately honor KoD rate/deny semantics.
fn deliver(gate: &mut UtcTrustPreflight, app: &mut RadarApp, now: u64, outcome: SntpOutcome) {
    let accepted = matches!(outcome, SntpOutcome::Observation(_));
    let result = live_utc::apply(gate, app, now, outcome);
    if accepted {
        assert_eq!(result, Some(ObservationOutcome::Accepted));
    }
}

struct Display {
    screen: Screen,
    sender: LineSender,
    coast: CoastLayer,
    period: PeriodCache,
}

impl Display {
    fn new(app: &RadarApp) -> Self {
        Self {
            screen: Screen::new(),
            sender: LineSender::new(),
            coast: CoastLayer::prepare(app.observer()),
            period: PeriodCache::new(),
        }
    }

    fn draw(&mut self, app: &RadarApp) {
        draw_cached(
            &mut self.screen,
            app,
            Layout::wide(),
            false,
            &mut self.coast,
            &mut self.period,
        )
        .unwrap();
    }

    fn send(&mut self, phase: bool, full: bool) -> Kind {
        self.sender
            .send(phase, &self.screen.pixels, full, |_| Ok::<_, ()>(()))
            .unwrap()
            .kind
    }
}

fn frame_time(utc_us: i64) -> NaiveDateTime {
    DateTime::from_timestamp_micros(utc_us).unwrap().naive_utc()
}

#[test]
fn cold_boot_network_wait_sync_holdover_expiry_and_recovery() {
    let mut app = fixture_app("iss-ground-20", 0.0, 0.0, 5.0);
    let elements_epoch_us = app.time().unwrap().and_utc().timestamp_micros();
    let mut gate = UtcTrustPreflight::prototype();
    let mut display = Display::new(&app);

    // Boot must not display the fixture's pre-populated historical frame.
    assert_eq!(
        gate.next(&mut app, 0, false),
        LifecycleAction::RenderNoFrame
    );
    assert!(app.time().is_none());
    display.draw(&app);
    let no_frame = display.screen.pixels;
    let phase = gate.next_phase();
    assert_eq!(display.send(phase, true), Kind::Full);
    gate.no_frame_succeeded(52_000, phase);

    // The network is pending: the display owner can still poll independently.
    assert_eq!(
        gate.next(&mut app, 552_000, true),
        LifecycleAction::Scheduled(DiagnosticAction::IdleVcom)
    );
    let phase = gate.next_phase();
    display.sender.idle(phase, |_| Ok::<_, ()>(())).unwrap();
    gate.transfer_succeeded(552_100, phase, false);
    assert_eq!(
        gate.next(&mut app, 8_052_000, true),
        LifecycleAction::Scheduled(DiagnosticAction::RefreshRetained)
    );
    let phase = gate.next_phase();
    assert_eq!(display.send(phase, true), Kind::Full);
    gate.transfer_succeeded(8_104_000, phase, true);
    assert_eq!(display.screen.pixels, no_frame);
    assert!(app.time().is_none());

    let sent = 9_000_000;
    let received = sent + 20_000;
    deliver(
        &mut gate,
        &mut app,
        received,
        screened(&reply(elements_epoch_us, 0, 2), sent, received),
    );
    let LifecycleAction::Scheduled(DiagnosticAction::Frame { sample, full }) =
        gate.next(&mut app, received, true)
    else {
        panic!("first trusted observation must request a fresh frame");
    };
    assert_eq!(sample.utc_unix_us, elements_epoch_us + 10_500);
    app.update(frame_time(sample.utc_unix_us)).unwrap();
    display.draw(&app);
    assert_ne!(display.screen.pixels, no_frame);
    let phase = gate.next_phase();
    let kind = display.send(phase, full);
    gate.transfer_succeeded(received + 40_000, phase, kind == Kind::Full);
    assert!(app.selected().is_some());

    // Missing replies cannot renew the anchor. Holdover still samples actual
    // frame-start UTC and ends at H, not at network disconnect or reconnection.
    let near_expiry = sent + 10_000 + 3_599_000_000;
    gate.no_renewal(&mut app, near_expiry);
    let LifecycleAction::Scheduled(DiagnosticAction::Frame { sample, .. }) =
        gate.next(&mut app, near_expiry, true)
    else {
        panic!("still inside prototype holdover");
    };
    assert_eq!(sample.utc_unix_us, elements_epoch_us + 3_599_000_500);
    let expiry = sent + 10_000 + 3_600_000_000;
    assert_eq!(
        gate.next(&mut app, expiry, true),
        LifecycleAction::RenderNoFrame
    );
    assert!(app.time().is_none());
    assert!(app.selected().is_none());
    display.draw(&app);
    assert_eq!(display.screen.pixels, no_frame);

    // No stale tracking transfer at expiry, even if the panel previously had
    // tracking pixels; replacement is credited only after the full send.
    let phase = gate.next_phase();
    assert_eq!(display.send(phase, true), Kind::Full);
    gate.no_frame_succeeded(expiry + 52_000, phase);
    let resync_sent = expiry + 200_000;
    let resync_received = resync_sent + 20_000;
    deliver(
        &mut gate,
        &mut app,
        resync_received,
        screened(
            &reply(elements_epoch_us + 3_601_200_000, 0, 2),
            resync_sent,
            resync_received,
        ),
    );
    let LifecycleAction::Scheduled(DiagnosticAction::Frame { sample, .. }) =
        gate.next(&mut app, resync_received, true)
    else {
        panic!("resync must use the new anchor immediately");
    };
    assert_eq!(sample.utc_unix_us, elements_epoch_us + 3_601_210_500);
    // A source-validated >100 ms jump discards the old anchor. The next
    // sample uses the new UTC, not an old fixture offset or missed grid tick.
    app.update(frame_time(sample.utc_unix_us)).unwrap();
    display.draw(&app);
    assert_ne!(display.screen.pixels, no_frame);
    let phase = gate.next_phase();
    let kind = display.send(phase, true);
    gate.transfer_succeeded(resync_received + 40_000, phase, kind == Kind::Full);
}

#[test]
fn rejected_reply_revocation_and_failed_replacement_do_not_leak_tracking() {
    let mut app = fixture_app("iss-ground-20", 0.0, 0.0, 5.0);
    let epoch_us = app.time().unwrap().and_utc().timestamp_micros();
    let mut display = Display::new(&app);
    let mut gate = UtcTrustPreflight::prototype();
    assert_eq!(
        gate.next(&mut app, 0, false),
        LifecycleAction::RenderNoFrame
    );
    let sent = 1_000_000;
    let received = sent + 20_000;
    deliver(
        &mut gate,
        &mut app,
        received,
        screened(&reply(epoch_us, 0, 2), sent, received),
    );
    let LifecycleAction::Scheduled(DiagnosticAction::Frame { sample, full }) =
        gate.next(&mut app, received, false)
    else {
        panic!("first sync bypasses unsent NO FRAME");
    };
    app.update(frame_time(sample.utc_unix_us)).unwrap();
    display.draw(&app);
    let phase = gate.next_phase();
    let kind = display.send(phase, full);
    gate.transfer_succeeded(received + 40_000, phase, kind == Kind::Full);

    // An unmatched packet and a matched too-uncertain packet cannot replace
    // or renew the accepted anchor. A matched LI=1 revokes it immediately.
    let mut unmatched = reply(epoch_us, 0, 2);
    unmatched[24] ^= 1;
    let ignored = screened(&unmatched, 2_000_000, 2_020_000);
    assert!(matches!(ignored, SntpOutcome::Ignored(_)));
    deliver(&mut gate, &mut app, 2_020_000, ignored);
    let poor = screened(&reply(epoch_us, 0, 2), 3_000_000, 3_500_000);
    assert!(matches!(poor, SntpOutcome::Observation(_)));
    assert_eq!(
        gate.observe(
            &mut app,
            3_500_000,
            match poor {
                SntpOutcome::Observation(o) => o,
                _ => unreachable!(),
            }
        ),
        ObservationOutcome::NoRenewal
    );
    assert!(app.time().is_some());
    let kod = screened(&reply(epoch_us, 0, 0), 3_700_000, 3_720_000);
    assert!(matches!(kod, SntpOutcome::KissOfDeath(_)));
    deliver(&mut gate, &mut app, 3_720_000, kod);
    assert!(app.time().is_some());
    deliver(
        &mut gate,
        &mut app,
        4_020_000,
        screened(&reply(epoch_us, 1, 2), 4_000_000, 4_020_000),
    );
    assert_eq!(
        gate.next(&mut app, 4_020_000, true),
        LifecycleAction::RenderNoFrame
    );
    assert!(app.time().is_none());
    display.draw(&app);
    let no_frame = display.screen.pixels;
    let phase = gate.next_phase();
    let mut writes = 0;
    assert!(
        display
            .sender
            .send(phase, &display.screen.pixels, true, |_| {
                writes += 1;
                if writes == 2 { Err(()) } else { Ok(()) }
            })
            .is_err()
    );
    gate.transfer_failed();
    assert_eq!(gate.next_phase(), phase);
    assert_eq!(
        gate.next(&mut app, 4_100_000, false),
        LifecycleAction::RenderNoFrame
    );
    assert_eq!(display.send(phase, true), Kind::Full);
    gate.no_frame_succeeded(4_152_000, phase);
    assert_eq!(display.screen.pixels, no_frame);
    assert_eq!(
        gate.next(&mut app, 4_152_001, true),
        LifecycleAction::Scheduled(DiagnosticAction::Wait)
    );
}
