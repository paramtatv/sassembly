use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub name: String,
    pub dependencies: Vec<String>,
    pub source_hash: String,
}

#[derive(Debug, Clone)]
pub struct Manifest {
    pub targets: HashMap<String, Target>,
}

pub struct Builder {
    // Maps a content hash to its compiled output path/identifier
    pub cache: HashMap<String, String>,
}

/// `W-274`: `new()` takes no arguments, so `Default` is the same
/// constructor under the name the language expects. Written rather than
/// allowed, because `clippy::new_without_default` is asking for an
/// interface and not for silence.
impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Recursively builds a target and returns its output cache entry.
    pub fn build_target(
        &mut self,
        manifest: &Manifest,
        target_name: &str,
        visited: &mut HashSet<String>,
    ) -> Result<String, String> {
        if visited.contains(target_name) {
            return Err(format!(
                "Circular dependency detected involving target '{}'",
                target_name
            ));
        }

        let target = manifest
            .targets
            .get(target_name)
            .ok_or_else(|| format!("Target '{}' not found", target_name))?;

        visited.insert(target_name.to_string());

        // Resolve dependencies first
        let mut dep_hashes = Vec::new();
        for dep in &target.dependencies {
            let dep_output = self.build_target(manifest, dep, visited)?;
            dep_hashes.push(dep_output);
        }

        visited.remove(target_name);

        // Combine dependency hashes with the target's own source hash to create a unique build fingerprint
        let fingerprint = format!("{}:[{}]", target.source_hash, dep_hashes.join(","));

        // Check if this fingerprint exists in the cache
        if let Some(cached_output) = self.cache.get(&fingerprint) {
            return Ok(cached_output.clone());
        }

        // Simulate compilation (in reality, run typecheck, emit, etc.)
        let compiled_output = format!("out_{}", fingerprint);

        // Store in cache
        self.cache
            .insert(fingerprint.clone(), compiled_output.clone());

        Ok(compiled_output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_cache_hit() {
        let mut manifest = Manifest {
            targets: HashMap::new(),
        };

        manifest.targets.insert(
            "libA".to_string(),
            Target {
                name: "libA".to_string(),
                dependencies: vec![],
                source_hash: "hashA".to_string(),
            },
        );

        manifest.targets.insert(
            "app".to_string(),
            Target {
                name: "app".to_string(),
                dependencies: vec!["libA".to_string()],
                source_hash: "hashApp".to_string(),
            },
        );

        let mut builder = Builder::new();

        // First build
        let out1 = builder
            .build_target(&manifest, "app", &mut HashSet::new())
            .unwrap();
        assert_eq!(out1, "out_hashApp:[out_hashA:[]]");
        assert_eq!(builder.cache.len(), 2);

        // Second build should hit cache without increasing cache size
        let out2 = builder
            .build_target(&manifest, "app", &mut HashSet::new())
            .unwrap();
        assert_eq!(out1, out2);
        assert_eq!(builder.cache.len(), 2);
    }

    #[test]
    fn test_circular_dependency() {
        let mut manifest = Manifest {
            targets: HashMap::new(),
        };

        manifest.targets.insert(
            "libA".to_string(),
            Target {
                name: "libA".to_string(),
                dependencies: vec!["libB".to_string()],
                source_hash: "hashA".to_string(),
            },
        );

        manifest.targets.insert(
            "libB".to_string(),
            Target {
                name: "libB".to_string(),
                dependencies: vec!["libA".to_string()],
                source_hash: "hashB".to_string(),
            },
        );

        let mut builder = Builder::new();
        let res = builder.build_target(&manifest, "libA", &mut HashSet::new());
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Circular dependency"));
    }
}
