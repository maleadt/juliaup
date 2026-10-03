//! Build variants of released Julia versions, e.g. `1.13+nogpl`.
//!
//! The versions database does not list variant builds yet (`nightlies.json`
//! does for nightlies, see `nightlies_db`). Until it does, juliaup derives them:
//! the `nogpl` builds (Julia built with `USE_GPL_LIBS=0`) of a release are
//! published to a separate bucket under a fixed layout, so every standard build
//! for a platform that has `nogpl` builds gets a `nogpl` counterpart, and every
//! channel `C` (`C~arch`) pointing at such a build gets a `C+nogpl`
//! (`C+nogpl~arch`) channel. Whether a given build actually exists is only known
//! when downloading it, see [`missing_build_error`].

use crate::channel_name::ChannelName;
use crate::jsonstructs_versionsdb::{
    JuliaupVersionDB, JuliaupVersionDBChannel, JuliaupVersionDBVersion,
};
use crate::operations::HttpStatusError;
use anyhow::Error;
use semver::Version;

pub const NOGPL: &str = "nogpl";

const NOGPL_SERVER: &str = "https://julialang-nogpl.s3.amazonaws.com/";

/// The variant builds that exist for releases, as `+variant` suffixes.
pub const RELEASE_VARIANTS: &[&str] = &["+nogpl"];

/// The `nogpl` directory and file platform for the platform of a standard
/// build, given by the `<arch>.<platform>` build metadata of its database key
/// (e.g. `x64.linux.gnu`).
fn nogpl_platform(build: &str) -> Option<(&'static str, &'static str)> {
    let (arch, platform) = build.split_once('.')?;
    match (arch, platform) {
        ("x64", "linux.gnu") => Some(("linuxnogpl", "x86_64")),
        ("x64", "apple.darwin14") => Some(("macosnogpl", "x86_64")),
        ("aarch64", "apple.darwin14") => Some(("macosnogpl", "aarch64")),
        ("x64", "w64.mingw32") => Some(("windowsnogpl", "x86_64")),
        _ => None,
    }
}

/// The database key and download URL of the `nogpl` build corresponding to a
/// standard database key like `1.13.1+0.x64.linux.gnu`.
fn nogpl_build(key: &str) -> Option<(String, String)> {
    let (version, metadata) = key.split_once('+')?;
    let (_, build) = metadata.split_once('.')?;
    let (os, arch) = nogpl_platform(build)?;
    let parsed = Version::parse(version).ok()?;
    let url = format!(
        "{NOGPL_SERVER}bin-nogpl/{os}/{arch}/{}.{}/julia-{version}-{os}-{arch}.tar.gz",
        parsed.major, parsed.minor
    );
    Some((format!("{key}.{NOGPL}"), url))
}

/// The `nogpl` channel name for a database channel (`1.13` → `1.13+nogpl`,
/// `1.13~x64` → `1.13+nogpl~x64`).
fn nogpl_channel(channel: &str) -> String {
    match channel.split_once('~') {
        Some((base, arch)) => format!("{base}+{NOGPL}~{arch}"),
        None => format!("{channel}+{NOGPL}"),
    }
}

/// Adds the derived variant builds and channels to `db`. Existing entries are
/// never replaced.
pub fn add_to(db: &mut JuliaupVersionDB) {
    let builds: Vec<(String, String)> = db
        .available_versions
        .keys()
        .filter_map(|key| nogpl_build(key))
        .collect();
    for (key, url) in builds {
        db.available_versions
            .entry(key)
            .or_insert(JuliaupVersionDBVersion { url_path: url });
    }

    let channels: Vec<(String, String)> = db
        .available_channels
        .iter()
        .filter(|(channel, _)| !channel.contains('+'))
        .filter_map(|(channel, target)| {
            let (key, _) = nogpl_build(&target.version)?;
            Some((nogpl_channel(channel), key))
        })
        .collect();
    for (channel, version) in channels {
        db.available_channels
            .entry(channel)
            .or_insert(JuliaupVersionDBChannel { version });
    }
}

/// Whether `channel` names a release variant rather than a standard channel.
pub fn is_variant_channel(channel: &str) -> bool {
    ChannelName::parse(channel)
        .is_ok_and(|name| !name.is_direct_download() && !name.variants.is_empty())
}

/// Whether the database `version` is a derived variant build.
pub fn is_variant_version(version: &str) -> bool {
    version.ends_with(&format!(".{NOGPL}"))
}

/// Explains a failed download of the variant build `version`: the derived
/// builds are not guaranteed to exist, so a 404 (or the 403 that S3 answers
/// for missing keys without list access) means the build was never published.
pub fn missing_build_error(version: &str, err: Error) -> Error {
    let missing = err
        .chain()
        .find_map(|cause| cause.downcast_ref::<HttpStatusError>())
        .is_some_and(|status| matches!(status.status, 403 | 404));
    if !missing {
        return err;
    }
    let (base, metadata) = version.split_once('+').unwrap_or((version, ""));
    let platform = metadata
        .split_once('.')
        .and_then(|(_, build)| build.strip_suffix(&format!(".{NOGPL}")))
        .and_then(nogpl_platform)
        .map(|(os, arch)| format!(" for {}/{}", os.trim_end_matches(NOGPL), arch))
        .unwrap_or_default();
    err.context(format!(
        "No nogpl build of Julia {base}{platform} is currently available."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn db() -> JuliaupVersionDB {
        let versions = [
            (
                "1.13.1+0.x64.linux.gnu",
                "bin/linux/x64/1.13/julia-1.13.1-linux-x86_64.tar.gz",
            ),
            (
                "1.13.1+0.x86.linux.gnu",
                "bin/linux/x86/1.13/julia-1.13.1-linux-i686.tar.gz",
            ),
            (
                "1.13.0-rc1+0.aarch64.apple.darwin14",
                "bin/mac/aarch64/1.13/julia-1.13.0-rc1-macaarch64.tar.gz",
            ),
        ];
        let channels = [
            ("release", "1.13.1+0.x64.linux.gnu"),
            ("1.13~x64", "1.13.1+0.x64.linux.gnu"),
            ("1.13~x86", "1.13.1+0.x86.linux.gnu"),
            ("1.13.0-rc1", "1.13.0-rc1+0.aarch64.apple.darwin14"),
        ];
        JuliaupVersionDB {
            available_versions: versions
                .iter()
                .map(|(k, u)| {
                    (
                        k.to_string(),
                        JuliaupVersionDBVersion {
                            url_path: u.to_string(),
                        },
                    )
                })
                .collect(),
            available_channels: channels
                .iter()
                .map(|(c, v)| {
                    (
                        c.to_string(),
                        JuliaupVersionDBChannel {
                            version: v.to_string(),
                        },
                    )
                })
                .collect::<HashMap<_, _>>(),
            version: "1.0.0".to_string(),
        }
    }

    #[test]
    fn derives_nogpl_builds_and_channels() {
        let mut db = db();
        add_to(&mut db);

        assert_eq!(
            db.available_versions["1.13.1+0.x64.linux.gnu.nogpl"].url_path,
            "https://julialang-nogpl.s3.amazonaws.com/bin-nogpl/linuxnogpl/x86_64/1.13/julia-1.13.1-linuxnogpl-x86_64.tar.gz"
        );
        assert_eq!(
            db.available_versions["1.13.0-rc1+0.aarch64.apple.darwin14.nogpl"].url_path,
            "https://julialang-nogpl.s3.amazonaws.com/bin-nogpl/macosnogpl/aarch64/1.13/julia-1.13.0-rc1-macosnogpl-aarch64.tar.gz"
        );
        assert_eq!(
            db.available_channels["release+nogpl"].version,
            "1.13.1+0.x64.linux.gnu.nogpl"
        );
        assert_eq!(
            db.available_channels["1.13+nogpl~x64"].version,
            "1.13.1+0.x64.linux.gnu.nogpl"
        );
        assert!(db.available_channels.contains_key("1.13.0-rc1+nogpl"));

        // No nogpl builds for 32-bit Linux.
        assert!(!db
            .available_versions
            .contains_key("1.13.1+0.x86.linux.gnu.nogpl"));
        assert!(!db.available_channels.contains_key("1.13+nogpl~x86"));

        // Idempotent.
        let (versions, channels) = (db.available_versions.len(), db.available_channels.len());
        add_to(&mut db);
        assert_eq!(db.available_versions.len(), versions);
        assert_eq!(db.available_channels.len(), channels);
    }

    #[test]
    fn derived_channels_parse_as_variants() {
        for channel in ["release+nogpl", "1.13+nogpl~x64", "1.13.0-rc1+nogpl"] {
            let name = ChannelName::parse(channel).unwrap();
            assert_eq!(name.variants, [NOGPL]);
            assert_eq!(name.to_string(), channel);
            assert!(is_variant_channel(channel));
        }
        assert!(!is_variant_channel("release"));
        assert!(!is_variant_channel("nightly+nogpl"));
    }

    #[test]
    fn explains_missing_builds() {
        let version = "1.13.1+0.x64.linux.gnu.nogpl";
        let missing = Error::new(HttpStatusError {
            url: "https://example.com/x.tar.gz".to_string(),
            status: 404,
        })
        .context("Failed to install");
        assert_eq!(
            missing_build_error(version, missing).to_string(),
            "No nogpl build of Julia 1.13.1 for linux/x86_64 is currently available."
        );

        let other = Error::new(HttpStatusError {
            url: "https://example.com/x.tar.gz".to_string(),
            status: 500,
        });
        assert!(missing_build_error(version, other)
            .to_string()
            .contains("HTTP status 500"));
    }
}
