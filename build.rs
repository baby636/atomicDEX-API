use chrono::DateTime;
use gstuff::slurp;
use regex::Regex;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::from_utf8;

fn path2s(path: PathBuf) -> String {
    path.to_str()
        .unwrap_or_else(|| panic!("Non-stringy path {:?}", path))
        .into()
}

/// SuperNET's root.
fn root() -> PathBuf {
    let super_net = Path::new(env!("CARGO_MANIFEST_DIR"));
    let super_net = match super_net.canonicalize() {
        Ok(p) => p,
        Err(err) => panic!("Can't canonicalize {:?}: {}", super_net, err),
    };
    // On Windows we're getting these "\\?\" paths from canonicalize but they aren't any good for CMake.
    if cfg!(windows) {
        let s = path2s(super_net);
        let stripped = match s.strip_prefix(r"\\?\") {
            Some(stripped) => stripped,
            None => &s,
        };
        Path::new(stripped).into()
    } else {
        super_net
    }
}

/// This function ensures that we have the “MM_VERSION” and “MM_DATETIME” variables during the build.
///
/// The build script will usually help us by putting the MarketMaker version into the “MM_VERSION” file
/// and the corresponding ISO 8601 time into the “MM_DATETIME” file
///
/// For the nightly builds the version contains the short commit hash.
///
/// We're also trying to get the hash and the time from Git.
///
/// Git information isn't always available during the build (for instance, when a build server is used,
/// we might skip synchronizing the Git repository there),
/// but if it is, then we're going to check if the “MM_DATETIME” and the Git data match.
fn mm_version() -> String {
    // We fetch the actual git version here,
    // with `git log '--pretty=format:%h' -n 1` for the nightlies,
    // and a release tag when building from some kind of a stable branch,
    // though we should keep the ability for the tooling to provide the “MM_VERSION”
    // externally, because moving the entire ".git" around is not always practical.
    let mut version = "UNKNOWN".to_string();
    let mut command = Command::new("git");
    command.arg("log").arg("--pretty=format:%h").arg("-n1");
    if let Ok(go) = command.output() {
        if go.status.success() {
            version = from_utf8(&go.stdout).unwrap().trim().to_string();
            if !Regex::new(r"^\w+$").unwrap().is_match(&version) {
                panic!("{}", version)
            }
        }
    }

    let mm_version_p = root().join("MM_VERSION");
    let v_file = String::from_utf8(slurp(&mm_version_p)).unwrap();
    let v_file = v_file.trim().to_string();
    if version[..] != v_file[..] {
        // Create or update the MM_VERSION file in order to appease the Cargo dependency management.
        let mut mm_version_f = fs::File::create(&mm_version_p).unwrap();
        mm_version_f.write_all(version.as_bytes()).unwrap();
    }

    println!("cargo:rustc-env=MM_VERSION={}", version);

    let mut dt_git = None;
    let mut command = Command::new("git");
    command.arg("log").arg("--pretty=format:%cI").arg("-n1"); // ISO 8601
    if let Ok(go) = command.output() {
        if go.status.success() {
            let got = from_utf8(&go.stdout).unwrap().trim();
            let _dt_check = DateTime::parse_from_rfc3339(got).unwrap();
            dt_git = Some(got.to_string());
        }
    }

    let mm_datetime_p = root().join("MM_DATETIME");
    let dt_file = String::from_utf8(slurp(&mm_datetime_p)).unwrap();
    let dt_file = dt_file.trim().to_string();
    if let Some(ref dt_git) = dt_git {
        if dt_git[..] != dt_file[..] {
            // Create or update the “MM_DATETIME” file in order to appease the Cargo dependency management.
            let mut mm_datetime_f = fs::File::create(&mm_datetime_p).unwrap();
            mm_datetime_f.write_all(dt_git.as_bytes()).unwrap();
        }
    }

    println!("cargo:rustc-env=MM_DATETIME={}", dt_file);

    version
}

fn main() { mm_version(); }
