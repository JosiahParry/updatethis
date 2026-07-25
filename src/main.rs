use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use colored::Colorize;
use r_description::lossless::RDescription;
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

mod version;
mod workflow;

use version::{Version, VersionType};
use workflow::Workflow;

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
    /// Set the Version field to a specific version
    #[command(alias = "sv")]
    SetVersion {
        /// The version to set, as `x.y.z` or `x.y.z.w`
        version: Version,
        /// Path to the package root (defaults to the current directory)
        path: Option<PathBuf>,
        /// Set the version even if it is not greater than the current one
        #[arg(short, long)]
        force: bool,
    },
    /// Write a GitHub Actions workflow that sets the version from git tags
    Init {
        /// Path to the repository root (defaults to the current directory)
        path: Option<PathBuf>,
        /// Overwrite an existing workflow
        #[arg(short, long)]
        force: bool,
    },
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
            None => Version::DEFAULT,
            Some(mut v) => {
                bump_type.increment(&mut v);
                v
            }
        };

        self.inner.set_version(&new_version.to_string());

        Ok(new_version)
    }

    /// Set the `Version` field to `new_version`.
    ///
    /// Unless `force` is set, the new version must be greater than the
    /// current one.
    fn set_version(&mut self, new_version: Version, force: bool) -> anyhow::Result<()> {
        if let (Some(current), false) = (self.version()?, force) {
            if new_version <= current {
                bail!(
                    "{new_version} is not greater than the current version {current}; \
                     pass --force to set it anyway"
                );
            }
        }

        self.inner.set_version(&new_version.to_string());

        Ok(())
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
        Command::SetVersion {
            version,
            path,
            force,
        } => {
            let root = path.unwrap_or_else(|| PathBuf::from("."));

            let mut description = Description::read(&root)?;

            let old_version = description.version()?;
            description.set_version(version, force)?;

            let path = description.write()?;

            let old = old_version
                .map(|v| v.to_string())
                .unwrap_or_else(|| "none".to_string());

            println!(
                "{} {} {} {}",
                "set version from".bold(),
                old.red(),
                "->".dimmed(),
                version.to_string().green().bold()
            );
            println!(
                "{} {}",
                "Wrote".dimmed(),
                path.display().to_string().dimmed()
            );
        }
        Command::Init { path, force } => {
            let root = path.unwrap_or_else(|| PathBuf::from("."));

            let path = Workflow::write(&root, force)?;

            println!(
                "{} {}",
                "Wrote".bold(),
                path.display().to_string().green().bold()
            );
            println!(
                "{}",
                "Tag a release to set the version, e.g. `git sv tag`".dimmed()
            );
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
            Version::DEFAULT
        );
        assert_eq!(
            description.version().expect("valid"),
            Some(Version::DEFAULT)
        );
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
    fn sets_a_greater_version() {
        let mut description = fixture();

        description
            .set_version(Version::new(2, 0, 0), false)
            .expect("2.0.0 is greater than 1.0.0");

        assert_eq!(
            description.version().expect("valid"),
            Some(Version::new(2, 0, 0))
        );
    }

    #[test]
    fn rejects_a_lesser_or_equal_version() {
        for target in ["1.0.0", "0.9.9"] {
            let mut description = fixture();
            let target: Version = target.parse().expect("valid");

            assert!(
                description.set_version(target, false).is_err(),
                "{target} should be rejected"
            );

            // the rejected write leaves the current version untouched
            assert_eq!(
                description.version().expect("valid"),
                Some(Version::new(1, 0, 0))
            );
        }
    }

    #[test]
    fn force_overrides_the_ordering_check() {
        let mut description = fixture();

        description
            .set_version(Version::new(0, 1, 0), true)
            .expect("force ignores the check");

        assert_eq!(
            description.version().expect("valid"),
            Some(Version::new(0, 1, 0))
        );
    }

    #[test]
    fn sets_a_version_when_none_is_present() {
        let mut description: Description = "Package: nover\n".parse().expect("parses");

        description
            .set_version(Version::new(1, 0, 0), false)
            .expect("nothing to compare against");

        assert_eq!(
            description.version().expect("valid"),
            Some(Version::new(1, 0, 0))
        );
    }
}
