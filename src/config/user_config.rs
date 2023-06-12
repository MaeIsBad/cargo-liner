use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use home::cargo_home;
use serde::{Deserialize, Serialize};

use super::Package;

/// Represents the user's configuration deserialized from its file.
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct UserConfig {
    /// The name-to-setting map for the `packages` section of the config.
    pub packages: BTreeMap<String, Package>,
}

impl UserConfig {
    /// The default name for the configuration file in Cargo's home.
    pub const FILE_NAME: &str = "liner.toml";

    /// Returns the [`PathBuf`] pointing to the associated configuration file.
    pub fn file_path() -> Result<PathBuf> {
        Ok(cargo_home()?.join(Self::FILE_NAME))
    }

    /// Deserializes the user's configuration file and returns the result.
    ///
    /// It may fail on multiple occasions: if Cargo's home may not be found, if
    /// the file does not exist, if it cannot be read from or if it is malformed.
    pub fn parse_file() -> Result<Self> {
        Ok(toml::from_str::<Self>(&fs::read_to_string(Self::file_path()?)?)?.self_update(true))
    }

    /// Serializes the configuration and saves it to the default file.
    ///
    /// It creates the file if it does not already exist. If it already exists,
    /// contents will be enterily overwritten. Just as [`Self::parse_file`], it
    /// may fail on several occasions.
    pub fn save_file(&self) -> Result<()> {
        fs::write(Self::file_path()?, self.to_string_pretty()?)?;
        Ok(())
    }

    /// Converts the config to a pretty TOML string with literal strings disabled.
    fn to_string_pretty(&self) -> Result<String> {
        let mut dst = String::new();
        self.serialize(toml::Serializer::pretty(&mut dst).pretty_string_literal(false))?;
        Ok(dst)
    }

    /// Enable or disable self-updating.
    ///
    /// If `sup` is `true` and the current crate is not already contained in
    /// the configured packages, then it will add it, otherwise remove it.
    pub fn self_update(mut self, sup: bool) -> Self {
        if sup {
            if !self.packages.contains_key(clap::crate_name!()) {
                self.packages
                    .insert(clap::crate_name!().to_owned(), Package::SIMPLE_STAR);
            }
        } else {
            self.packages.remove(clap::crate_name!());
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use semver::VersionReq;
    use std::iter;

    #[test]
    fn test_deser_userconfig_empty_iserr() {
        assert!(toml::from_str::<UserConfig>("").is_err());
    }

    #[test]
    fn test_deser_userconfig_no_packages() {
        assert_eq!(
            toml::from_str::<UserConfig>("[packages]\n").unwrap(),
            UserConfig::default(),
        );
    }

    #[test]
    fn test_deser_userconfig_simple_versions() {
        assert_eq!(
            toml::from_str::<UserConfig>(
                r#"
                    [packages]
                    a = "1.2.3"
                    b = "1.2"
                    c = "1"
                    d = "*"
                    e = "1.*"
                    f = "1.2.*"
                    g = "~1.2"
                    h = "~1"
                "#,
            )
            .unwrap(),
            UserConfig {
                packages: [
                    ("a", "1.2.3"),
                    ("b", "1.2"),
                    ("c", "1"),
                    ("d", "*"),
                    ("e", "1.*"),
                    ("f", "1.2.*"),
                    ("g", "~1.2"),
                    ("h", "~1")
                ]
                .into_iter()
                .map(|(name, version)| (
                    name.to_owned(),
                    Package::Simple(VersionReq::parse(version).unwrap()),
                ))
                .collect::<BTreeMap<_, _>>(),
            }
        );
    }

    #[test]
    fn test_deser_userconfig_detailed_requirements() {
        let mut packages = toml::from_str::<UserConfig>(
            r#"
                    [packages]
                    a = "1.2.3"
                    b = { version = "1.2", features = [ "foo" ] }
                    c = { version = "1.2", features = [ "foo" ], default-features = false }
                    d = { version = "1.2", all-features = true }
                    e = { version = "1.2" }
                "#,
        )
        .unwrap()
        .packages
        .into_iter()
        .map(|(name, req)| {
            let version = req.version().to_string();
            let features = req.features().to_owned();
            (
                name,
                (
                    version,
                    features,
                    req.all_features(),
                    req.default_features(),
                ),
            )
        })
        .collect::<Vec<_>>();
        packages.sort_by_key(|(k, _)| k.clone());

        let packages: Vec<_> = packages.into_iter().map(|(_, v)| v).collect();

        let expected = [
            ("^1.2.3".to_string(), vec![], false, true),
            ("^1.2".to_string(), vec!["foo".to_string()], false, true),
            ("^1.2".to_string(), vec!["foo".to_string()], false, false),
            ("^1.2".to_string(), vec![], true, true),
            ("^1.2".to_string(), vec![], false, true),
        ]
        .into_iter()
        .collect::<Vec<_>>();

        assert_eq!(packages, expected);
    }

    #[test]
    fn test_userconfig_tostringpretty_no_packages() {
        assert_eq!(
            UserConfig::default().to_string_pretty().unwrap(),
            "[packages]\n",
        );
    }

    #[test]
    fn test_userconfig_tostringpretty_simple_versions() {
        assert_eq!(
            UserConfig {
                packages: [
                    ("a", "1.2.3"),
                    ("b", "1.2"),
                    ("c", "1"),
                    ("d", "*"),
                    ("e", "1.*"),
                    ("f", "1.2.*"),
                    ("g", "~1.2"),
                    ("h", "~1"),
                ]
                .into_iter()
                .map(|(name, version)| (
                    name.to_owned(),
                    Package::Simple(VersionReq::parse(version).unwrap()),
                ))
                .collect::<BTreeMap<_, _>>(),
            }
            .to_string_pretty()
            .unwrap(),
            indoc!(
                r#"
                    [packages]
                    a = "^1.2.3"
                    b = "^1.2"
                    c = "^1"
                    d = "*"
                    e = "1.*"
                    f = "1.2.*"
                    g = "~1.2"
                    h = "~1"
                "#,
            ),
        );
    }

    #[test]
    fn test_userconfig_selfupdate_enable() {
        assert_eq!(
            UserConfig::default().self_update(true).packages,
            iter::once(("cargo-liner".to_owned(), Package::SIMPLE_STAR))
                .collect::<BTreeMap<_, _>>(),
        );
    }

    #[test]
    fn test_userconfig_selfupdate_enable_noreplace() {
        let pkgs = iter::once((
            "cargo-liner".to_owned(),
            Package::Simple(VersionReq::parse("1.2.3").unwrap()),
        ))
        .collect::<BTreeMap<_, _>>();

        assert_eq!(
            UserConfig {
                packages: pkgs.clone(),
            }
            .self_update(true)
            .packages,
            pkgs,
        );
    }

    #[test]
    fn test_userconfig_selfupdate_disable_star() {
        assert_eq!(
            UserConfig::default()
                .self_update(true)
                .self_update(false)
                .packages,
            BTreeMap::new(),
        );
    }

    #[test]
    fn test_userconfig_selfupdate_disable_nostar() {
        assert_eq!(
            UserConfig {
                packages: iter::once((
                    "cargo-liner".to_owned(),
                    Package::Simple(VersionReq::parse("1.2.3").unwrap()),
                ))
                .collect::<BTreeMap<_, _>>(),
            }
            .self_update(false)
            .packages,
            BTreeMap::new(),
        );
    }
}