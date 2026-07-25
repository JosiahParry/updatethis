use anyhow::{Context, bail};
use std::path::{Path, PathBuf};

/// The GitHub Actions workflow that keeps DESCRIPTION in sync with git tags.
pub struct Workflow;

impl Workflow {
    /// Where the workflow is written, relative to the repository root.
    const PATH: &'static str = ".github/workflows/set-version.yml";

    /// The workflow itself.
    ///
    /// Triggers on both three- and four-component tags: GitHub's tag filters
    /// are globs rather than regexes, and `*` does not match `.`, so the dev
    /// version form needs its own pattern.
    const TEMPLATE: &'static str = r#"# Keep the DESCRIPTION Version field in sync with git tags.
#
# Tag a release with `git sv tag` (or by hand) and this writes the version
# into DESCRIPTION and commits it.
name: set-version

on:
  push:
    tags:
      - "v*.*.*"
      - "v*.*.*.*"

jobs:
  set-version:
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v6
      - uses: JosiahParry/updatethis@v1
"#;

    /// Write the workflow into the repository rooted at `root`.
    ///
    /// Refuses to overwrite an existing workflow unless `force` is set.
    pub fn write(root: &Path, force: bool) -> anyhow::Result<PathBuf> {
        let path = root.join(Self::PATH);

        if path.exists() && !force {
            bail!(
                "{} already exists; pass --force to overwrite it",
                path.display()
            );
        }

        let parent = path
            .parent()
            .expect("the workflow path always has a parent directory");

        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;

        std::fs::write(&path, Self::TEMPLATE)
            .with_context(|| format!("Failed to write {}", path.display()))?;

        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_template_covers_both_tag_shapes() {
        assert!(Workflow::TEMPLATE.contains(r#""v*.*.*""#));
        assert!(Workflow::TEMPLATE.contains(r#""v*.*.*.*""#));
    }

    #[test]
    fn the_template_grants_write_permission() {
        // pushing the version commit fails without it
        assert!(Workflow::TEMPLATE.contains("contents: write"));
    }
}
