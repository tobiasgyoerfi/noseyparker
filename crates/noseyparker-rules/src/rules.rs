use anyhow::{bail, Context, Result};
use ignore::types::TypesBuilder;
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, debug_span};

use crate::{util, Rule};

// -------------------------------------------------------------------------------------------------
// Rules
// -------------------------------------------------------------------------------------------------
#[derive(Serialize, Deserialize)]
pub struct Rules {
    pub rules: Vec<Rule>,
}

impl Rules {
    pub fn from_paths_and_contents<'a, I: IntoIterator<Item=(&'a Path, &'a [u8])>>(iterable: I) -> Result<Self> {
        let mut rules = Rules { rules: Vec::new() };
        for (path, contents) in iterable.into_iter() {
            let rs: Self = serde_yaml::from_reader(contents)
                .with_context(|| format!("Failed to load rules YAML from {}", path.display()))?;
            rules.extend(rs);
        }

        Ok(rules)
    }

    /// Create an empty collection of rules.
    pub fn new() -> Self {
        Rules { rules: Vec::new() }
    }

    /// Load rules from the given paths, which may refer either to YAML files or to directories.
    pub fn from_paths<P: AsRef<Path>, I: IntoIterator<Item=P>>(paths: I) -> Result<Self> {
        let mut num_paths = 0;
        let mut rules = Rules::new();
        for input in paths {
            num_paths += 1;
            let input = input.as_ref();
            if input.is_file() {
                rules.extend(Rules::from_yaml_file(input)?);
            } else if input.is_dir() {
                rules.extend(Rules::from_directory(input)?);
            } else {
                bail!("Unhandled input type: {} is neither a file nor directory", input.display());
            }
        }
        debug!("Loaded {} rules from {num_paths} paths", rules.len());
        Ok(rules)
    }

    /// Load rules from the given YAML file.
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let _span = debug_span!("Rules::from_yaml_file", "{}", path.display()).entered();
        let rules: Self = util::load_yaml_file(path)
            .with_context(|| format!("Failed to load rules YAML from {}", path.display()))?;
        debug!("Loaded {} rules from {}", rules.len(), path.display());
        Ok(rules)
    }

    /// Load rules from the given YAML files.
    pub fn from_yaml_files<P: AsRef<Path>, I: IntoIterator<Item=P>>(paths: I) -> Result<Self> {
        let mut num_paths = 0;
        let mut rules = Vec::new();
        for path in paths {
            num_paths += 1;
            rules.extend(Rules::from_yaml_file(path.as_ref())?);
        }
        debug!("Loaded {} rules from {num_paths} files", rules.len());
        Ok(Rules { rules })
    }

    /// Load rules from YAML files found recursively within the given directory.
    pub fn from_directory<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let _span = debug_span!("Rules::from_directory", "{}", path.display()).entered();

        let yaml_types = TypesBuilder::new().add_defaults().select("yaml").build()?;

        let walker = WalkBuilder::new(path)
            .types(yaml_types)
            .follow_links(true)
            .standard_filters(false)
            .build();
        let mut yaml_files = Vec::new();
        for entry in walker {
            let entry = entry?;
            if entry.file_type().map_or(false, |t| !t.is_dir()) {
                yaml_files.push(entry.into_path());
            }
        }
        yaml_files.sort();
        debug!("Found {} rules files to load within {}", yaml_files.len(), path.display());

        Self::from_yaml_files(&yaml_files)
    }

    /// How many rules are in this collection?
    #[inline]
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Is this collection of rules empty?
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, Rule> {
        self.rules.iter()
    }
}

/// Creates an empty collection of rules.
impl Default for Rules {
    fn default() -> Self {
        Self::new()
    }
}

impl Extend<Rule> for Rules {
    fn extend<T: IntoIterator<Item = Rule>>(&mut self, iter: T) {
        self.rules.extend(iter);
    }
}

impl IntoIterator for Rules {
    type Item = Rule;
    type IntoIter = <Vec<Rule> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.rules.into_iter()
    }
}
