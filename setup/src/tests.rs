use super::*;

fn req(r: &Request) -> ([u8; MAX_FRAME], usize) {
    let mut b = [0; MAX_FRAME];
    let n = encode_request(r, &mut b).unwrap();
    (b, n)
}

fn resp(r: &Response) -> ([u8; MAX_FRAME], usize) {
    let mut b = [0; MAX_FRAME];
    let n = encode_response(r, &mut b).unwrap();
    (b, n)
}

const CONFIGURE: Configure = Configure {
    ssid: "home",
    passphrase: "password1",
    lat_deg: 51.5,
    lon_deg: -0.125,
    alt_m: 35.0,
};

const INFO: Info = Info {
    build: 12,
    version: "0.1.12-beta",
    channel: Channel::Beta,
    mac: [0x80, 0x45, 0x6b, 0x2a, 0x45, 0x08],
    flags: Info::WIFI_STORED | Info::SITE_STORED,
    reason: SetupReason::Button,
    ssid: "home",
    last_fetch_unix_s: 1_794_000_000,
    failed_build: 0,
};

/// Reference bytes for the iOS app's tests: change only with a new version.
#[test]
fn fixed_vectors() {
    assert_eq!(&req(&Request::GetInfo).0[..2], &[0x01, 0x01]);
    assert_eq!(&req(&Request::Scan).0[..2], &[0x01, 0x02]);
    let (b, n) = req(&Request::Configure(CONFIGURE));
    #[rustfmt::skip]
    let expected: &[u8] = &[
        0x01, 0x03,
        4, b'h', b'o', b'm', b'e',
        9, b'p', b'a', b's', b's', b'w', b'o', b'r', b'd', b'1',
        0, 0, 0, 0, 0, 0xC0, 0x49, 0x40, // 51.5
        0, 0, 0, 0, 0, 0, 0xC0, 0xBF,    // -0.125
        0, 0, 0, 0, 0, 0x80, 0x41, 0x40, // 35.0
    ];
    assert_eq!(&b[..n], expected);

    let (b, n) = resp(&Response::Info(INFO));
    #[rustfmt::skip]
    let expected: &[u8] = &[
        0x01, 0x81,
        12, 0, 0, 0,
        11, b'0', b'.', b'1', b'.', b'1', b'2', b'-', b'b', b'e', b't', b'a',
        1,
        0x80, 0x45, 0x6b, 0x2a, 0x45, 0x08,
        0b11,
        3,
        4, b'h', b'o', b'm', b'e',
        0x80, 0x44, 0xEE, 0x6A, 0, 0, 0, 0, // 1_794_000_000
        0, 0, 0, 0,
    ];
    assert_eq!(&b[..n], expected);

    let network = Response::Network(Network {
        rssi_dbm: -61,
        security: Security::Personal,
        ssid: "home",
    });
    let (b, n) = resp(&network);
    assert_eq!(&b[..n], &[0x01, 0x82, 0xC3, 0, 4, b'h', b'o', b'm', b'e']);
    let (b, n) = resp(&Response::ScanDone { count: 3, ok: true });
    assert_eq!(&b[..n], &[0x01, 0x83, 3, 1]);
    let (b, n) = resp(&Response::Testing);
    assert_eq!(&b[..n], &[0x01, 0x84]);
    let (b, n) = resp(&Response::ConfigureResult(ConfigureResult::WrongPassphrase));
    assert_eq!(&b[..n], &[0x01, 0x85, 5]);
    let error = Response::Error {
        code: ErrorCode::UnknownType,
        request_type: 0x09,
    };
    let (b, n) = resp(&error);
    assert_eq!(&b[..n], &[0x01, 0x86, 2, 0x09]);
}

#[test]
fn every_message_round_trips() {
    for r in [
        Request::GetInfo,
        Request::Scan,
        Request::Configure(CONFIGURE),
    ] {
        let (b, n) = req(&r);
        let f = decode_request(&b[..n]).unwrap();
        assert_eq!((f.version, f.message), (VERSION, r));
    }
    let responses = [
        Response::Info(INFO),
        Response::Network(Network {
            rssi_dbm: -90,
            security: Security::Unsupported,
            ssid: "café ☕",
        }),
        Response::ScanDone {
            count: 0,
            ok: false,
        },
        Response::Testing,
        Response::ConfigureResult(ConfigureResult::Saved),
        Response::Error {
            code: ErrorCode::Malformed,
            request_type: 0x02,
        },
    ];
    for r in responses {
        let (b, n) = resp(&r);
        let f = decode_response(&b[..n]).unwrap();
        assert_eq!((f.version, f.message), (VERSION, r));
    }
}

#[test]
fn longest_frames_fit() {
    let ssid = "s".repeat(SSID_MAX);
    let pass = "p".repeat(PASSPHRASE_MAX);
    let version = "v".repeat(VERSION_MAX);
    let c = Configure {
        ssid: &ssid,
        passphrase: &pass,
        ..CONFIGURE
    };
    assert!(req(&Request::Configure(c)).1 <= MAX_FRAME);
    let i = Info {
        version: &version,
        ssid: &ssid,
        ..INFO
    };
    assert!(resp(&Response::Info(i)).1 <= MAX_FRAME);
}

#[test]
fn encode_refuses_long_strings_and_small_buffers() {
    let ssid = "s".repeat(SSID_MAX + 1);
    let c = Configure {
        ssid: &ssid,
        ..CONFIGURE
    };
    let mut b = [0; MAX_FRAME];
    assert_eq!(
        encode_request(&Request::Configure(c), &mut b),
        Err(EncodeError)
    );
    let mut small = [0; 10];
    assert_eq!(
        encode_request(&Request::Configure(CONFIGURE), &mut small),
        Err(EncodeError)
    );
    assert_eq!(
        encode_request(&Request::GetInfo, &mut [0; 1]),
        Err(EncodeError)
    );
}

#[test]
fn malformed_requests_are_refused() {
    use DecodeError::*;
    let (b, n) = req(&Request::Configure(CONFIGURE));
    // Every cut of a Configure frame is truncated, never a partial message.
    for cut in 0..n {
        assert_eq!(decode_request(&b[..cut]), Err(Truncated), "cut {cut}");
    }
    assert_eq!(decode_request(&[0, 1]), Err(Version));
    // Bad UTF-8 in the SSID.
    let mut bad = b;
    bad[3] = 0xFF;
    assert_eq!(decode_request(&bad[..n]), Err(BadString));
    // An SSID length over the limit, with the bytes present.
    let mut long = [0u8; MAX_FRAME];
    long[..3].copy_from_slice(&[1, 3, 33]);
    long[3..36].fill(b'a');
    assert_eq!(decode_request(&long), Err(BadString));
    // A response type sent as a request.
    assert_eq!(decode_request(&[1, 0x81]), Err(UnknownType(0x81)));
    assert_eq!(UnknownType(0x81).code(), ErrorCode::Malformed);
    assert_eq!(Truncated.code(), ErrorCode::Malformed);
    assert_eq!(type_of(&[1]), 0);
    assert_eq!(type_of(&[1, 3]), 3);
}

#[test]
fn malformed_responses_are_refused() {
    assert_eq!(decode_response(&[1, 0x85, 99]), Err(DecodeError::BadValue));
    assert_eq!(decode_response(&[1, 0x83, 1]), Err(DecodeError::Truncated));
    assert_eq!(
        decode_response(&[1, 0x01]),
        Err(DecodeError::UnknownType(0x01))
    );
}

/// A newer client (version 2, appended fields, new types) against this
/// device, and a newer device's responses against this client.
#[test]
fn newer_peers_keep_working() {
    let (mut b, n) = req(&Request::Configure(CONFIGURE));
    b[0] = 2;
    b[n..n + 4].copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
    let f = decode_request(&b[..n + 4]).unwrap();
    assert_eq!(f.version, 2);
    assert_eq!(f.message, Request::Configure(CONFIGURE));
    assert_eq!(
        decode_request(&[2, 0x01, 7, 7]).unwrap().message,
        Request::GetInfo
    );

    let unknown = decode_request(&[2, 0x10, 1, 2, 3]).unwrap_err();
    assert_eq!(unknown, DecodeError::UnknownType(0x10));
    assert_eq!(unknown.code(), ErrorCode::UnknownType);

    let (mut b, n) = resp(&Response::Info(INFO));
    b[0] = 2;
    b[n] = 0x42;
    let f = decode_response(&b[..n + 1]).unwrap();
    assert_eq!((f.version, f.message), (2, Response::Info(INFO)));
}

#[test]
fn configure_check_matches_the_settings_rules() {
    assert_eq!(CONFIGURE.check(), Ok(()));
    let with = |f: fn(&mut Configure)| {
        let mut c = CONFIGURE;
        f(&mut c);
        c.check()
    };
    use ConfigureResult::*;
    assert_eq!(with(|c| c.ssid = ""), Err(InvalidSsid));
    assert_eq!(
        with(|c| c.ssid = "123456789012345678901234567890123"),
        Err(InvalidSsid)
    );
    assert_eq!(with(|c| c.passphrase = "short"), Err(InvalidPassphrase));
    assert_eq!(
        with(|c| c.passphrase = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
        Ok(())
    );
    assert_eq!(
        with(|c| c.passphrase = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdeg"),
        Err(InvalidPassphrase)
    );
    assert_eq!(with(|c| c.lat_deg = 90.5), Err(InvalidSite));
    assert_eq!(with(|c| c.lon_deg = f64::NAN), Err(InvalidSite));
    assert_eq!(with(|c| c.alt_m = 9_001.0), Err(InvalidSite));
    assert_eq!(with(|c| c.lon_deg = -180.0), Ok(()));
}

#[test]
fn uuid_bytes_are_little_endian() {
    let b = uuid_le(SERVICE_UUID);
    assert_eq!(b[15], 0x6f);
    assert_eq!(b[0], 0x40);
    assert_eq!(&b[12..14], &[0x10, 0x00]);
    assert!(str_eq(SERVICE_UUID, "6f760010-8a3e-4c1d-9b7a-0b5e1c2d3a40"));
    assert!(!str_eq(SERVICE_UUID, REQUEST_UUID));
}
