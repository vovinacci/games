//! A game updating itself from its releases in this repository (`docs/RELEASING.md`).
//!
//! A check reads the game's update channel: `latest.txt` of the fixed release `<game>-channel`, one line with the tag
//! of the newest release, and its minisign signature. Updating downloads `SHA256SUMS` of that release and its
//! signature, downloads this system's file, checks its hash, replaces the game and offers to start it again. Every
//! signature is checked with the public key built in here (`assets/release-key.pub`); a file that fails a check is
//! never installed. Work on the network runs on a thread of its own.
//!
//! The key signs for every game of the repository, so a signature says only where a file is from. Which game and
//! which version it is for is read from what was signed: the channel must name a tag of this game, newer than the
//! running one, and the checksum is looked up under the file name with game and version in it.
//!
//! Off without the `self-update` feature (a packager updates the game by its own means) and for a build that is not
//! a release: version `0.0.0`, or one with `+` in it.

#![cfg_attr(
    all(not(feature = "self-update"), not(test)),
    expect(
        dead_code,
        reason = "without the feature only the status and its words are used"
    )
)]

use std::sync::{Arc, Mutex};

use crate::t;

/// The repository whose releases the games update from.
pub const REPO: &str = "vovinacci/games";

/// Which game runs, and which build of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Build {
    /// The name the releases use: `bounty` of `bounty-v0.1.0`.
    pub game: &'static str,
    /// `X.Y.Z`, as Cargo has it.
    pub version: &'static str,
}

impl Build {
    /// Whether this build can update itself.
    #[must_use]
    pub fn updates(self) -> bool {
        cfg!(feature = "self-update") && self.version != "0.0.0" && !self.version.contains('+')
    }

    /// The game's name as its packages write it: `Bounty`.
    fn title(self) -> String {
        let mut letters = self.game.chars();
        letters.next().map_or_else(String::new, |first| {
            first.to_uppercase().chain(letters).collect()
        })
    }

    /// The version a line of the channel names for this game, without `v`: `0.1.0` of `bounty-v0.1.0`. Nothing
    /// for another game's tag or anything that is not a release tag.
    fn version_named(self, channel: &str) -> Option<String> {
        let version = channel
            .trim_end()
            .strip_prefix(self.game)?
            .strip_prefix("-v")?;
        numbers(version).map(|_| version.to_owned())
    }
}

/// The three numbers of `X.Y.Z`, digits only.
fn numbers(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.split('.').map(|part| {
        let digits = !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
        digits.then(|| part.parse().ok()).flatten()
    });
    let found = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(found)
}

/// `latest` is a newer version than `current` (both `X.Y.Z`).
fn is_newer(latest: &str, current: &str) -> bool {
    match (numbers(latest), numbers(current)) {
        (Some(latest), Some(current)) => latest > current,
        _ => false,
    }
}

/// What the updater is doing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Idle,
    Checking,
    UpToDate,
    /// The check got no answer (no network, GitHub down, a signature that does not match).
    CheckFailed(String),
    /// A newer release, its version without `v`.
    Available {
        version: String,
    },
    Downloading {
        version: String,
    },
    /// Installed; the game has to start again to be it.
    Installed {
        version: String,
    },
    InstallFailed(String),
}

/// Checks and updates in the background; the game asks for [`Updater::status`] or [`Updater::line`] every frame.
pub struct Updater {
    build: Build,
    status: Arc<Mutex<Status>>,
}

impl Updater {
    #[must_use]
    pub fn new(build: Build) -> Self {
        Self {
            build,
            status: Arc::new(Mutex::new(Status::Idle)),
        }
    }

    #[must_use]
    pub fn status(&self) -> Status {
        self.status
            .lock()
            .map_or(Status::Idle, |status| status.clone())
    }

    fn set(&self, status: Status) {
        if let Ok(mut held) = self.status.lock() {
            *held = status;
        }
    }

    /// Starts a check in the background, unless the build does not update itself or work is under way.
    pub fn check(&self) {
        let busy = matches!(self.status(), Status::Checking | Status::Downloading { .. });
        if !self.build.updates() || busy {
            return;
        }
        self.set(Status::Checking);
        #[cfg(feature = "self-update")]
        {
            let (build, status) = (self.build, Arc::clone(&self.status));
            std::thread::spawn(move || {
                let result = match net::latest(build) {
                    Ok(version) if is_newer(&version, build.version) => {
                        Status::Available { version }
                    }
                    Ok(_) => Status::UpToDate,
                    Err(problem) => Status::CheckFailed(problem),
                };
                if let Ok(mut held) = status.lock() {
                    *held = result;
                }
            });
        }
    }

    /// Downloads, checks and installs the offered version in the background.
    pub fn install(&self) {
        let Status::Available { version } = self.status() else {
            return;
        };
        self.set(Status::Downloading {
            version: version.clone(),
        });
        #[cfg(feature = "self-update")]
        {
            let (build, status) = (self.build, Arc::clone(&self.status));
            std::thread::spawn(move || {
                let result = match net::install(build, &version) {
                    Ok(()) => Status::Installed { version },
                    Err(problem) => Status::InstallFailed(problem),
                };
                if let Ok(mut held) = status.lock() {
                    *held = result;
                }
            });
        }
    }

    /// What the one update key does now: installs an offered version, or starts the installed one. Nothing while
    /// there is nothing to do.
    pub fn act(&self) {
        match self.status() {
            Status::Available { .. } => self.install(),
            Status::Installed { .. } => restart(),
            _ => {}
        }
    }

    /// The line a game shows about updates, in the player's language, with the key of [`Updater::act`] (U) where
    /// it does something. Nothing while there is no news: a check that failed stays silent, as the player did not
    /// ask for it.
    #[must_use]
    pub fn line(&self) -> Option<String> {
        match self.status() {
            Status::Available { version } => Some(t!("update-available", v = version)),
            Status::Downloading { version } => Some(t!("update-downloading", v = version)),
            Status::Installed { version } => Some(t!("update-installed", v = version)),
            Status::InstallFailed(_) => Some(t!("update-failed")),
            Status::Idle | Status::Checking | Status::UpToDate | Status::CheckFailed(_) => None,
        }
    }
}

/// `<game> --update`: checks and installs from the command line, in English. Returns the exit code of the process.
#[must_use]
pub fn command_line(build: Build) -> i32 {
    let (name, running) = (build.title(), build.version);
    if !build.updates() {
        println!("{name} {running} does not update itself: it is not a released build.");
        return 1;
    }
    #[cfg(feature = "self-update")]
    {
        let latest = match net::latest(build) {
            Ok(version) => version,
            Err(problem) => {
                eprintln!("Could not check for updates: {problem}");
                return 1;
            }
        };
        if !is_newer(&latest, running) {
            println!("{name} {running} is the newest version.");
            return 0;
        }
        println!("Updating {name} {running} to {latest}...");
        match net::install(build, &latest) {
            Ok(()) => {
                println!("{name} {latest} is installed.");
                0
            }
            Err(problem) => {
                eprintln!("The update failed: {problem}");
                1
            }
        }
    }
    #[cfg(not(feature = "self-update"))]
    1
}

/// Starts the installed version, then ends this one.
pub fn restart() -> ! {
    #[cfg(feature = "self-update")]
    if let Some(program) = net::installed_program() {
        let _ = std::process::Command::new(program).spawn();
    }
    std::process::exit(0);
}

#[cfg(feature = "self-update")]
mod net {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use sha2::{Digest, Sha256};

    use super::{Build, REPO};

    /// The key the releases are signed with (a minisign public key file).
    pub(super) const PUBLIC_KEY: &str = include_str!("../assets/release-key.pub");
    /// The largest download taken (a release file is a few MB).
    const MAX_DOWNLOAD: u64 = 100 * 1024 * 1024;

    fn get(build: Build, url: &str, seconds: u64) -> Result<Vec<u8>, String> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(seconds)))
            .user_agent(format!("{}/{}", build.game, build.version))
            .build()
            .into();
        agent
            .get(url)
            .call()
            .map_err(|e| format!("{url}: {e}"))?
            .body_mut()
            .with_config()
            .limit(MAX_DOWNLOAD)
            .read_to_vec()
            .map_err(|e| format!("{url}: {e}"))
    }

    fn get_text(build: Build, url: &str) -> Result<String, String> {
        String::from_utf8(get(build, url, 30)?).map_err(|e| format!("{url}: {e}"))
    }

    /// A file of the release with this tag.
    fn release_file(tag: &str, file: &str) -> String {
        format!("https://github.com/{REPO}/releases/download/{tag}/{file}")
    }

    /// `text` as it was signed: fails unless `signature` (a minisign signature file) is the one `public_key` (a
    /// minisign public key file) makes for it.
    pub(super) fn signed<'a>(
        public_key: &str,
        text: &'a str,
        signature: &str,
    ) -> Result<&'a str, String> {
        let key = minisign_verify::PublicKey::decode(public_key).map_err(|e| e.to_string())?;
        let signature =
            minisign_verify::Signature::decode(signature).map_err(|e| format!("signature: {e}"))?;
        key.verify(text.as_bytes(), &signature, false)
            .map_err(|_| "the signature is not the release key's".to_owned())?;
        Ok(text)
    }

    /// The version of the game's newest release, without `v`, as its channel names it.
    pub fn latest(build: Build) -> Result<String, String> {
        let channel = format!("{}-channel", build.game);
        let text = get_text(build, &release_file(&channel, "latest.txt"))?;
        let signature = get_text(build, &release_file(&channel, "latest.txt.minisig"))?;
        let line = signed(PUBLIC_KEY, &text, &signature).map_err(|e| format!("latest.txt: {e}"))?;
        build
            .version_named(line)
            .ok_or_else(|| format!("latest.txt names no release of {}", build.game))
    }

    /// The SHA-256 a checksum list gives for `file`.
    pub(super) fn listed_hash(sums: &str, file: &str) -> Result<String, String> {
        sums.lines()
            .find_map(|line| {
                let (hash, name) = line.split_once("  ")?;
                (name == file).then(|| hash.to_lowercase())
            })
            .ok_or_else(|| format!("{file} is not in SHA256SUMS"))
    }

    pub(super) fn sha256_hex(data: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        Sha256::digest(data)
            .iter()
            .flat_map(|b| [HEX[usize::from(b >> 4)], HEX[usize::from(b & 15)]])
            .map(char::from)
            .collect()
    }

    /// The release file for this system and where it goes.
    enum Target {
        /// A plain program replacing the running one (Linux, Windows).
        Program(String),
        /// An AppImage replacing the AppImage file the game runs from.
        AppImage { file: String, path: PathBuf },
        /// A zip with the game's `.app`, replacing the bundle the game runs from.
        MacApp { file: String, bundle: PathBuf },
    }

    fn target(build: Build, tag: &str) -> Result<Target, String> {
        let arch = std::env::consts::ARCH;
        let (game, title) = (build.game, build.title());
        if cfg!(target_os = "macos") {
            let program = std::env::current_exe().map_err(|e| e.to_string())?;
            // .../Bounty.app/Contents/MacOS/bounty
            let bundle = program
                .ancestors()
                .nth(3)
                .filter(|b| b.extension().is_some_and(|e| e == "app"))
                .ok_or_else(|| format!("the game is not running from {title}.app"))?;
            Ok(Target::MacApp {
                file: format!("{tag}-macos.zip"),
                bundle: bundle.to_path_buf(),
            })
        } else if let Some(path) = std::env::var_os("APPIMAGE") {
            let version = tag.strip_prefix(game).unwrap_or(tag);
            Ok(Target::AppImage {
                file: format!("{title}{version}-{arch}.AppImage"),
                path: PathBuf::from(path),
            })
        } else if cfg!(windows) {
            Ok(Target::Program(format!("{tag}-windows-{arch}.exe")))
        } else {
            Ok(Target::Program(format!("{tag}-linux-{arch}")))
        }
    }

    pub fn install(build: Build, version: &str) -> Result<(), String> {
        let tag = format!("{}-v{version}", build.game);
        let target = target(build, &tag)?;
        let file = match &target {
            Target::Program(file) | Target::AppImage { file, .. } | Target::MacApp { file, .. } => {
                file.clone()
            }
        };
        let sums = get_text(build, &release_file(&tag, "SHA256SUMS"))?;
        let signature = get_text(build, &release_file(&tag, "SHA256SUMS.minisig"))?;
        let sums = signed(PUBLIC_KEY, &sums, &signature).map_err(|e| format!("SHA256SUMS: {e}"))?;
        let expected = listed_hash(sums, &file)?;
        let data = get(build, &release_file(&tag, &file), 300)?;
        if sha256_hex(&data) != expected {
            return Err(format!("{file}: the checksum does not match"));
        }

        match target {
            Target::Program(_) => {
                let name = format!("{}-update-{}", build.game, std::process::id());
                let new = std::env::temp_dir().join(name);
                write_program(&new, &data)?;
                let result = self_replace::self_replace(&new).map_err(|e| e.to_string());
                let _ = fs::remove_file(&new);
                result
            }
            Target::AppImage { path, .. } => {
                let new = path.with_extension("update");
                write_program(&new, &data)?;
                fs::rename(&new, &path).map_err(|e| e.to_string())
            }
            Target::MacApp { bundle, .. } => replace_bundle(build, &bundle, &data),
        }
    }

    fn write_program(path: &Path, data: &[u8]) -> Result<(), String> {
        fs::write(path, data).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o755))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Unpacks the release zip next to the running bundle (the same volume, so a rename works) with the system's
    /// `ditto`, then swaps the bundles.
    fn replace_bundle(build: Build, bundle: &Path, zip: &[u8]) -> Result<(), String> {
        let app = format!("{}.app", build.title());
        let parent = bundle
            .parent()
            .ok_or_else(|| format!("{app} has no folder above it"))?;
        let work = parent.join(format!(".{}-update-{}", build.game, std::process::id()));
        let _ = fs::remove_dir_all(&work);
        fs::create_dir_all(&work).map_err(|e| e.to_string())?;
        let result = (|| {
            let zip_path = work.join("update.zip");
            fs::write(&zip_path, zip).map_err(|e| e.to_string())?;
            let unpacked = std::process::Command::new("/usr/bin/ditto")
                .args(["-x", "-k"])
                .arg(&zip_path)
                .arg(&work)
                .status()
                .map_err(|e| e.to_string())?
                .success();
            if !unpacked {
                return Err("could not unpack the update".to_owned());
            }
            let new_bundle = fs::read_dir(&work)
                .map_err(|e| e.to_string())?
                .filter_map(|entry| entry.ok().map(|entry| entry.path().join(&app)))
                .find(|path| path.is_dir())
                .ok_or_else(|| format!("{app} is not in the update"))?;
            let old = work.join("old.app");
            fs::rename(bundle, &old).map_err(|e| e.to_string())?;
            if let Err(e) = fs::rename(&new_bundle, bundle) {
                let _ = fs::rename(&old, bundle);
                return Err(e.to_string());
            }
            Ok(())
        })();
        let _ = fs::remove_dir_all(&work);
        result
    }

    /// The program to start after an update: the same path as now, since the file or bundle was replaced in place.
    pub fn installed_program() -> Option<PathBuf> {
        match std::env::var_os("APPIMAGE") {
            Some(path) => Some(PathBuf::from(path)),
            None => std::env::current_exe().ok(),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        // A throwaway key pair, used only here; SUMS was signed with its secret key.
        const TEST_KEY: &str = "untrusted comment: minisign public key 2E4BB988727CB55F
RWRftXxyiLlLLmIi+pr7Z4bgCQOQSk6fkxA1syzsXpEckcKA46jYWKI9
";
        const SUMS: &str = "abc123  scorch-v9.9.9-linux-x86_64\n";
        const SIGNATURE: &str = "untrusted comment: signature from minisign secret key
RURftXxyiLlLLuc/1K6ikfyG1AUVf8/dLjHRAoEeLp6bf3DhZjIidTJnTJaWw1jaxEXCEe1TEz5Jc2ZexDH6ARTRnAHWg9m0yAc=
trusted comment: test
/YGpmmmfeh1lfchRq6/bfAl976Jxlrj1r44UEN3AuF08fDu6E1fNg7bf45pePsRsM+mq5nQTQ6g29hYefeYrDQ==
";

        #[test]
        fn a_signed_text_is_taken() {
            assert_eq!(signed(TEST_KEY, SUMS, SIGNATURE), Ok(SUMS));
        }

        #[test]
        fn a_changed_text_is_refused() {
            let forged = SUMS.replace("abc123", "def456");
            assert!(signed(TEST_KEY, &forged, SIGNATURE).is_err());
        }

        #[test]
        fn another_key_is_refused() {
            assert!(signed(PUBLIC_KEY, SUMS, SIGNATURE).is_err());
        }

        #[test]
        fn the_checksum_is_found_by_the_whole_file_name() {
            let sums =
                "aa11  bounty-v0.2.0-linux-x86_64.tar.gz\nBB22  bounty-v0.2.0-linux-x86_64\n";
            let hash = listed_hash(sums, "bounty-v0.2.0-linux-x86_64");
            assert_eq!(hash, Ok("bb22".to_owned()));
            assert!(listed_hash(sums, "bounty-v0.2.0-linux-aarch64").is_err());
        }

        #[test]
        fn sha256_is_lowercase_hex() {
            assert_eq!(
                sha256_hex(b"abc"),
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            );
        }

        #[test]
        fn the_release_key_parses() {
            assert!(minisign_verify::PublicKey::decode(PUBLIC_KEY).is_ok());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOUNTY: Build = Build {
        game: "bounty",
        version: "0.1.2",
    };

    #[test]
    fn versions_compare_by_number() {
        assert!(is_newer("0.1.10", "0.1.9"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.1.2", "0.1.2"));
        assert!(!is_newer("0.1.1", "0.1.2"));
        assert!(!is_newer("garbage", "0.1.2"));
        assert!(!is_newer("0.2.0-rc1", "0.1.2"));
        assert!(!is_newer("0.2.0.1", "0.1.2"));
    }

    #[test]
    fn the_channel_names_a_release_of_this_game_only() {
        let named = |line: &str| BOUNTY.version_named(line);
        assert_eq!(named("bounty-v0.2.0\n").as_deref(), Some("0.2.0"));
        assert_eq!(named("bounty-v10.20.30").as_deref(), Some("10.20.30"));
        // Signed with the same key, and still not this game's.
        assert_eq!(named("scorch-v9.9.9\n"), None);
        assert_eq!(named("bounty-channel\n"), None);
        assert_eq!(named("bounty-v0.2\n"), None);
        assert_eq!(named("bounty-v0.2.0\nbounty-v0.3.0\n"), None);
        assert_eq!(named("v0.2.0\n"), None);
        assert_eq!(named(""), None);
    }

    #[test]
    fn only_a_released_build_updates_itself() {
        let build = |version| Build {
            game: "bounty",
            version,
        };
        assert_eq!(build("0.1.0").updates(), cfg!(feature = "self-update"));
        assert!(!build("0.0.0").updates());
        assert!(!build("0.1.0+3.gabc1234").updates());
    }

    #[test]
    fn the_title_is_the_name_with_a_capital() {
        assert_eq!(BOUNTY.title(), "Bounty");
    }
}
