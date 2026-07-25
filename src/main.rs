use anyhow::{Context, bail};
use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use r_description::lossless::RDescription;
use std::{
    fmt,
    path::{Path, PathBuf},
    str::FromStr,
};

#[derive(Parser)]
#[command(
    name = "updatethis",
    version,
    about = "Bump the version of an R package"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Increment the Version field of a package's DESCRIPTION file
    Version {
        /// Which component of the version to bump
        version_type: VersionType,
        /// Path to the package root (defaults to the current directory)
        path: Option<PathBuf>,
    },
    /// Print the current version of a package
    #[command(alias = "cv")]
    Current {
        /// Path to the package root (defaults to the current directory)
        path: Option<PathBuf>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum VersionType {
    Major,
    Minor,
    Patch,
    Dev,
}

/// An R package version: `major.minor.patch` with an optional dev component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Version {
    major: u64,
    minor: u64,
    patch: u64,
    dev: Option<u64>,
}

const DEFAULT_VERSION: Version = Version {
    major: 0,
    minor: 1,
    patch: 0,
    dev: None,
};

/// The dev component R packages start at, by convention.
const FIRST_DEV: u64 = 9000;

impl Version {
    #[cfg(test)]
    fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
            dev: None,
        }
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

impl VersionType {
    fn increment(&self, version: &mut Version) {
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
                version.dev = Some(version.dev.map_or(FIRST_DEV, |dev| dev + 1));
            }
        }
    }
}

/// An R package DESCRIPTION file.
///
/// Backed by a lossless parse, so writing back out preserves the original
/// formatting — continuation lines, comments and all — and only the fields
/// we actually change are touched.
struct Description {
    inner: RDescription,
    /// Where the file was read from, if it came from disk.
    path: Option<PathBuf>,
}

impl FromStr for Description {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let inner = RDescription::from_str(s).map_err(|e| anyhow::anyhow!("{e}"))?;

        Ok(Self { inner, path: None })
    }
}

impl Description {
    /// Read and parse the DESCRIPTION file found at the root of a package.
    fn read(root: &Path) -> anyhow::Result<Self> {
        let path = root.join("DESCRIPTION");

        if !path.is_file() {
            bail!("No DESCRIPTION file found at {}", path.display());
        }

        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        let mut description: Self = contents.parse()?;
        description.path = Some(path);

        Ok(description)
    }

    /// Write the DESCRIPTION back to the file it was read from.
    fn write(&self) -> anyhow::Result<&Path> {
        let Some(path) = self.path.as_deref() else {
            bail!("This DESCRIPTION was not read from a file");
        };

        std::fs::write(path, self.inner.to_string())
            .with_context(|| format!("Failed to write {}", path.display()))?;

        Ok(path)
    }

    /// The package name.
    #[cfg(test)]
    fn package(&self) -> Option<String> {
        self.inner.package()
    }

    /// The parsed `Version` field.
    ///
    /// Returns `None` when the field is absent, and an error when it is
    /// present but not a valid version.
    fn version(&self) -> anyhow::Result<Option<Version>> {
        self.inner
            .version()
            .filter(|v| !v.trim().is_empty())
            .map(|v| v.parse().with_context(|| format!("Invalid Version: {v}")))
            .transpose()
    }

    /// Increment the `Version` field, defaulting it if absent.
    fn increment_version(&mut self, bump_type: VersionType) -> anyhow::Result<Version> {
        let new_version = match self.version()? {
            None => DEFAULT_VERSION,
            Some(mut v) => {
                bump_type.increment(&mut v);
                v
            }
        };

        self.inner.set_version(&new_version.to_string());

        Ok(new_version)
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Version { version_type, path } => {
            let root = path.unwrap_or_else(|| PathBuf::from("."));

            let mut description = Description::read(&root)?;

            let old_version = description.version()?;
            let new_version = description.increment_version(version_type)?;

            let path = description.write()?;

            let old = old_version
                .map(|v| v.to_string())
                .unwrap_or_else(|| "none".to_string());

            println!(
                "{} {} {} {}",
                "incremented version from".bold(),
                old.red(),
                "->".dimmed(),
                new_version.to_string().green().bold()
            );
            println!(
                "{} {}",
                "Wrote".dimmed(),
                path.display().to_string().dimmed()
            );
        }
        Command::Current { path } => {
            let root = path.unwrap_or_else(|| PathBuf::from("."));

            let description = Description::read(&root)?;

            let Some(version) = description.version()? else {
                bail!("No Version field found in DESCRIPTION");
            };

            println!("{}", version.to_string().green().bold());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/DESCRIPTION");

    fn fixture() -> Description {
        FIXTURE.parse().expect("fixture parses")
    }

    #[test]
    fn parses_fields() {
        let description = fixture();

        assert_eq!(description.package().as_deref(), Some("arcgisrouting"));
        assert_eq!(
            description.version().expect("valid"),
            Some(Version::new(1, 0, 0))
        );
    }

    #[test]
    fn missing_version_defaults() {
        let mut description: Description = "Package: nover\n".parse().expect("parses");

        assert_eq!(description.version().expect("valid"), None);
        assert_eq!(
            description
                .increment_version(VersionType::Patch)
                .expect("bumps"),
            DEFAULT_VERSION
        );
        assert_eq!(description.version().expect("valid"), Some(DEFAULT_VERSION));
    }

    #[test]
    fn invalid_version_is_an_error() {
        let mut description: Description = "Package: bad\nVersion: not-a-version\n"
            .parse()
            .expect("parses");

        assert!(description.version().is_err());
        assert!(description.increment_version(VersionType::Patch).is_err());
    }

    #[test]
    fn increments_and_writes_back() {
        let mut description = fixture();

        assert_eq!(
            description
                .increment_version(VersionType::Patch)
                .expect("bumps"),
            Version::new(1, 0, 1)
        );

        // the bump is persisted into the parsed DESCRIPTION
        assert_eq!(
            description.version().expect("valid"),
            Some(Version::new(1, 0, 1))
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
            assert_eq!(
                fixture()
                    .increment_version(bump_type)
                    .expect("bumps")
                    .to_string(),
                expected
            );
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
}
