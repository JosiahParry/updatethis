use anyhow::{Context, bail};
use clap::ValueEnum;
use std::{cmp::Ordering, fmt, str::FromStr};

/// Which component of a version to increment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum VersionType {
    Major,
    Minor,
    Patch,
    Dev,
}

impl VersionType {
    /// The dev component R packages start at, by convention.
    const FIRST_DEV: u64 = 9000;

    /// Increment `version` in place.
    ///
    /// Bumping any released component clears the dev component, matching the
    /// R convention where `1.0.0.9000` becomes `1.0.1` on release.
    pub fn increment(&self, version: &mut Version) {
        match self {
            VersionType::Major => {
                version.major += 1;
                version.minor = 0;
                version.patch = 0;
                version.dev = None;
            }
            VersionType::Minor => {
                version.minor += 1;
                version.patch = 0;
                version.dev = None;
            }
            VersionType::Patch => {
                version.patch += 1;
                version.dev = None;
            }
            VersionType::Dev => {
                version.dev = Some(version.dev.map_or(Self::FIRST_DEV, |dev| dev + 1));
            }
        }
    }
}

/// An R package version: `major.minor.patch` with an optional dev component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub dev: Option<u64>,
}

impl Version {
    /// The version assigned to a package that does not declare one.
    pub const DEFAULT: Version = Version {
        major: 0,
        minor: 1,
        patch: 0,
        dev: None,
    };

    /// A released version, with no dev component.
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
            dev: None,
        }
    }

    /// The components in comparison order.
    ///
    /// An absent dev component sorts below any present one, so `1.0.0` is
    /// older than `1.0.0.9000`.
    fn key(&self) -> (u64, u64, u64, Option<u64>) {
        (self.major, self.minor, self.patch, self.dev)
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key().cmp(&other.key())
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl FromStr for Version {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.trim().split(['.', '-']);

        let mut next = |field: &str| -> anyhow::Result<u64> {
            let Some(part) = parts.next() else {
                bail!("version `{s}` is missing a {field} component");
            };

            part.parse()
                .with_context(|| format!("`{part}` is not a valid {field} version"))
        };

        let major = next("major")?;
        let minor = next("minor")?;
        let patch = next("patch")?;

        let dev = match parts.next() {
            None => None,
            Some(part) => Some(
                part.parse()
                    .with_context(|| format!("`{part}` is not a valid dev version"))?,
            ),
        };

        if parts.next().is_some() {
            bail!("version `{s}` has too many components");
        }

        Ok(Self {
            major,
            minor,
            patch,
            dev,
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;

        if let Some(dev) = self.dev {
            write!(f, ".{dev}")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_displays_round_trip() {
        for raw in ["1.0.0", "1.0.0.9000", "0.1.0", "10.20.30.9999"] {
            let version: Version = raw.parse().expect("valid");
            assert_eq!(version.to_string(), raw);
        }
    }

    #[test]
    fn parses_dash_separated_dev_component() {
        assert_eq!(
            "1.0.0-9000".parse::<Version>().expect("valid"),
            Version {
                major: 1,
                minor: 0,
                patch: 0,
                dev: Some(9000)
            }
        );
    }

    #[test]
    fn rejects_malformed_versions() {
        for raw in ["1.0", "1.0.0.9000.1", "1.0.x", "", "not-a-version"] {
            assert!(raw.parse::<Version>().is_err(), "`{raw}` should not parse");
        }
    }

    #[test]
    fn orders_by_component_significance() {
        let ascending = [
            "0.1.0",
            "1.0.0",
            "1.0.0.9000",
            "1.0.0.9001",
            "1.0.1",
            "1.1.0",
            "2.0.0",
        ];

        for pair in ascending.windows(2) {
            let lower: Version = pair[0].parse().expect("valid");
            let higher: Version = pair[1].parse().expect("valid");

            assert!(lower < higher, "expected {lower} < {higher}");
        }
    }

    #[test]
    fn a_dev_version_outranks_its_release() {
        let release: Version = "1.0.0".parse().expect("valid");
        let dev: Version = "1.0.0.9000".parse().expect("valid");

        assert!(dev > release);
    }

    #[test]
    fn equivalent_separators_compare_equal() {
        assert_eq!(
            "2.5-1".parse::<Version>().expect("valid"),
            "2.5.1".parse::<Version>().expect("valid")
        );
    }

    #[test]
    fn increments_each_version_type() {
        let cases = [
            (VersionType::Major, "2.0.0"),
            (VersionType::Minor, "1.1.0"),
            (VersionType::Patch, "1.0.1"),
            (VersionType::Dev, "1.0.0.9000"),
        ];

        for (bump_type, expected) in cases {
            let mut version: Version = "1.0.0".parse().expect("valid");
            bump_type.increment(&mut version);

            assert_eq!(version.to_string(), expected);
        }
    }

    #[test]
    fn dev_increments_existing_dev_component() {
        let mut version: Version = "1.0.0.9000".parse().expect("valid");

        VersionType::Dev.increment(&mut version);

        assert_eq!(version.to_string(), "1.0.0.9001");
    }

    #[test]
    fn bumping_a_dev_version_drops_the_dev_component() {
        let mut version: Version = "1.0.0.9000".parse().expect("valid");

        VersionType::Patch.increment(&mut version);

        assert_eq!(version, Version::new(1, 0, 1));
    }

    #[test]
    fn incrementing_always_produces_a_greater_version() {
        let start: Version = "1.2.3".parse().expect("valid");

        for bump_type in [
            VersionType::Major,
            VersionType::Minor,
            VersionType::Patch,
            VersionType::Dev,
        ] {
            let mut bumped = start;
            bump_type.increment(&mut bumped);

            assert!(bumped > start, "expected {bumped} > {start}");
        }
    }
}
