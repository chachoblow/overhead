//! Release signing for OTA updates (0085). Runs on the user's Mac with the
//! release key; the private key never enters the repo, devbox or GitHub.
use std::{
    env, fs,
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    process::ExitCode,
};

use overhead_update::{Channel, Manifest, hex, parse_hex, public_key, sha256, sign, to_json};

const USAGE: &str = "Usage:
  overhead-release keygen KEY_FILE
      Make a new ed25519 secret key (64 hex chars, mode 0600; refuses to
      overwrite) and print its public key for firmware/release-keys.txt.
  overhead-release pubkey KEY_FILE
  overhead-release sign KEY_FILE CHANNEL BUILD VERSION IMAGE.bin OUT_DIR [MIN_SETTINGS]
      Copy IMAGE.bin to OUT_DIR/ota/images/BUILD.bin and write the signed
      OUT_DIR/ota/CHANNEL/manifest.json. CHANNEL is stable or beta; BUILD
      must be the build number compiled into the image (OVERHEAD_BUILD).
      MIN_SETTINGS defaults to 1.
  overhead-release verify MANIFEST.json PUBLIC_HEX [IMAGE.bin]
      Check the signature (and the image's length and SHA-256).";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("overhead-release: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let strs: Vec<&str> = args.iter().map(String::as_str).collect();
    match strs.as_slice() {
        ["keygen", path] => keygen(Path::new(path)),
        ["pubkey", path] => {
            println!("{}", hex(&public_key(&read_secret(Path::new(path))?)));
            Ok(())
        }
        ["sign", key, channel, build, version, image, out] => {
            sign_release(key, channel, build, version, image, out, "1")
        }
        ["sign", key, channel, build, version, image, out, min] => {
            sign_release(key, channel, build, version, image, out, min)
        }
        ["verify", manifest, public] => verify(manifest, public, None),
        ["verify", manifest, public, image] => verify(manifest, public, Some(image)),
        _ => Err(USAGE.into()),
    }
}

fn keygen(path: &Path) -> Result<(), String> {
    let mut secret = [0u8; 32];
    fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut secret))
        .map_err(|e| format!("reading /dev/urandom: {e}"))?;
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    writeln!(f, "{}", hex(&secret)).map_err(|e| e.to_string())?;
    println!("{}", hex(&public_key(&secret)));
    Ok(())
}

fn read_secret(path: &Path) -> Result<[u8; 32], String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_hex::<32>(text.trim()).map_err(|_| format!("{}: not a 64-hex-char key", path.display()))
}

fn sign_release(
    key: &str,
    channel: &str,
    build: &str,
    version: &str,
    image: &str,
    out: &str,
    min: &str,
) -> Result<(), String> {
    let secret = read_secret(Path::new(key))?;
    let channel = Channel::parse(channel).ok_or("CHANNEL must be stable or beta")?;
    let build: u32 = build.parse().map_err(|_| "BUILD must be a u32")?;
    let min_settings: u32 = min.parse().map_err(|_| "MIN_SETTINGS must be a u32")?;
    let bytes = fs::read(image).map_err(|e| format!("{image}: {e}"))?;
    let length = u32::try_from(bytes.len()).map_err(|_| "image too large")?;
    let m = Manifest {
        channel,
        build,
        version: version.into(),
        image: format!("images/{build}.bin"),
        length,
        sha256: sha256(&bytes),
        min_settings,
    };
    let json = to_json(&m, &sign(&m, &secret));
    // Check what will be published with the same code the device runs.
    overhead_update::verify(json.as_bytes(), &[public_key(&secret)])
        .map_err(|e| format!("manifest does not verify: {e}"))?;
    let ota = PathBuf::from(out).join("ota");
    let image_out = ota.join(&m.image);
    let manifest_out = ota.join(channel.as_str()).join("manifest.json");
    for p in [&image_out, &manifest_out] {
        fs::create_dir_all(p.parent().expect("has parent")).map_err(|e| e.to_string())?;
    }
    fs::write(&image_out, &bytes).map_err(|e| format!("{}: {e}", image_out.display()))?;
    fs::write(&manifest_out, &json).map_err(|e| format!("{}: {e}", manifest_out.display()))?;
    println!("{}\n{}", manifest_out.display(), image_out.display());
    Ok(())
}

fn verify(manifest: &str, public: &str, image: Option<&str>) -> Result<(), String> {
    let json = fs::read(manifest).map_err(|e| format!("{manifest}: {e}"))?;
    let key = parse_hex::<32>(public).map_err(|_| "PUBLIC_HEX must be 64 hex chars")?;
    let (m, _) = overhead_update::verify(&json, &[key]).map_err(|e| format!("refused: {e}"))?;
    println!(
        "ok: {} build {} version {} length {}",
        m.channel.as_str(),
        m.build,
        m.version,
        m.length
    );
    if let Some(image) = image {
        let bytes = fs::read(image).map_err(|e| format!("{image}: {e}"))?;
        let mut check = overhead_update::ImageCheck::new(&m);
        check
            .feed(&bytes)
            .and_then(|()| check.finish())
            .map_err(|e| format!("image refused: {e:?}"))?;
        println!("ok: image length and sha256 match");
    }
    Ok(())
}
