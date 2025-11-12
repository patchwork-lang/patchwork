/// Module resolution and dependency tracking for multi-file compilation
///
/// This module handles:
/// - Resolving import paths to file paths
/// - Building dependency graphs
/// - Detecting circular dependencies
/// - Tracking exports and making them available to importers

use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use patchwork_parser::ast::{Program, Item, ImportPath, ImportDecl};
use crate::error::{CompileError, Result};

/// A resolved module with its dependencies
#[derive(Debug, Clone)]
pub struct Module {
    /// Absolute path to the module file
    pub path: PathBuf,
    /// Module ID (relative path from project root, without .pw extension)
    pub id: String,
    /// Direct dependencies (module IDs this module imports)
    pub dependencies: Vec<String>,
    /// Source code (stored for re-parsing during compilation)
    pub source: String,
}

/// Exports from a module
#[derive(Debug, Clone)]
pub struct ModuleExports {
    /// Default export (worker, trait, or function name)
    pub default: Option<String>,
    /// Named exports (function names)
    pub named: Vec<String>,
}

/// Module resolver that handles import resolution and dependency tracking
pub struct ModuleResolver {
    /// Root directory for resolving relative imports
    root: PathBuf,
    /// Resolved modules by module ID
    modules: HashMap<String, Module>,
    /// Exports by module ID
    exports: HashMap<String, ModuleExports>,
    /// Modules currently being visited (for cycle detection)
    visiting: HashSet<String>,
}

impl ModuleResolver {
    /// Create a new module resolver with the given root directory
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            modules: HashMap::new(),
            exports: HashMap::new(),
            visiting: HashSet::new(),
        }
    }

    /// Resolve a module and all its dependencies starting from an entry point
    pub fn resolve(&mut self, entry_path: impl AsRef<Path>) -> Result<()> {
        let entry_path = entry_path.as_ref();
        let entry_id = self.path_to_module_id(entry_path)?;
        self.resolve_module(&entry_id, entry_path)?;
        Ok(())
    }

    /// Recursively resolve a module and its dependencies
    fn resolve_module(&mut self, module_id: &str, path: &Path) -> Result<()> {
        // Check for circular dependency
        if self.visiting.contains(module_id) {
            return Err(CompileError::CircularDependency(module_id.to_string()));
        }

        // Skip if already resolved
        if self.modules.contains_key(module_id) {
            return Ok(());
        }

        // Mark as visiting
        self.visiting.insert(module_id.to_string());

        // Read and parse the module
        let source = std::fs::read_to_string(path)?;
        let ast = patchwork_parser::parse(&source)
            .map_err(|e| CompileError::parse(path, e.to_string()))?;

        // Extract imports to find dependencies
        let dependencies = self.extract_dependencies(&ast, path)?;

        // Resolve each dependency recursively
        for (dep_id, dep_path) in &dependencies {
            self.resolve_module(dep_id, dep_path)?;
        }

        // Extract exports
        let exports = self.extract_exports(&ast);

        // Store the module (AST will be re-parsed during compilation from stored source)
        let module = Module {
            path: path.to_path_buf(),
            id: module_id.to_string(),
            dependencies: dependencies.iter().map(|(id, _)| id.clone()).collect(),
            source,
        };

        self.modules.insert(module_id.to_string(), module);
        self.exports.insert(module_id.to_string(), exports);

        // Unmark as visiting
        self.visiting.remove(module_id);

        Ok(())
    }

    /// Extract dependencies from a module's imports
    fn extract_dependencies(&self, ast: &Program, current_file: &Path) -> Result<Vec<(String, PathBuf)>> {
        let mut dependencies = Vec::new();

        for item in &ast.items {
            if let Item::Import(import_decl) = item {
                let deps = self.resolve_import(import_decl, current_file)?;
                dependencies.extend(deps);
            }
        }

        Ok(dependencies)
    }

    /// Resolve an import declaration to (module_id, file_path) pairs
    fn resolve_import(&self, import: &ImportDecl, current_file: &Path) -> Result<Vec<(String, PathBuf)>> {
        match &import.path {
            ImportPath::Simple(segments) => {
                // Check if this is a standard library import
                if segments.first() == Some(&"std") {
                    // Standard library imports don't need file resolution
                    return Ok(vec![]);
                }

                // Relative import: ./module or ../module
                let path = self.resolve_relative_path(segments, current_file)?;
                let module_id = self.path_to_module_id(&path)?;
                Ok(vec![(module_id, path)])
            }
            ImportPath::RelativeMulti(names) => {
                // Multi-import: ./{a, b, c} resolves to ./a.pw, ./b.pw, ./c.pw
                let current_dir = current_file.parent()
                    .ok_or_else(|| CompileError::ModuleResolution {
                        path: current_file.display().to_string(),
                        reason: "Cannot get parent directory".to_string(),
                    })?;

                let mut resolved = Vec::new();
                for name in names {
                    let mut path = current_dir.join(name);
                    path.set_extension("pw");

                    if !path.exists() {
                        return Err(CompileError::ModuleNotFound {
                            module: name.to_string(),
                            from: current_file.display().to_string(),
                        });
                    }

                    let module_id = self.path_to_module_id(&path)?;
                    resolved.push((module_id, path));
                }
                Ok(resolved)
            }
        }
    }

    /// Resolve a relative path to an absolute file path
    fn resolve_relative_path(&self, segments: &[&str], current_file: &Path) -> Result<PathBuf> {
        let current_dir = current_file.parent()
            .ok_or_else(|| CompileError::ModuleResolution {
                path: current_file.display().to_string(),
                reason: "Cannot get parent directory".to_string(),
            })?;

        let mut path = current_dir.to_path_buf();

        for segment in segments {
            if *segment == "." {
                // Stay in current directory
                continue;
            } else if *segment == ".." {
                // Go up one directory
                path.pop();
            } else {
                // Add directory or file
                path.push(segment);
            }
        }

        // Add .pw extension if not present
        if path.extension().is_none() {
            path.set_extension("pw");
        }

        if !path.exists() {
            return Err(CompileError::ModuleNotFound {
                module: segments.join("."),
                from: current_file.display().to_string(),
            });
        }

        Ok(path)
    }

    /// Convert a file path to a module ID (relative to root, without extension)
    fn path_to_module_id(&self, path: &Path) -> Result<String> {
        let abs_path = path.canonicalize()?;
        let abs_root = self.root.canonicalize()?;

        let rel_path = abs_path.strip_prefix(&abs_root)
            .map_err(|_| CompileError::ModuleResolution {
                path: path.display().to_string(),
                reason: format!("Path is not relative to root {}", abs_root.display()),
            })?;

        let id = rel_path.with_extension("")
            .to_string_lossy()
            .replace('\\', "/"); // Normalize path separators

        Ok(id)
    }

    /// Extract exports from a module's AST
    fn extract_exports(&self, ast: &Program) -> ModuleExports {
        let mut exports = ModuleExports {
            default: None,
            named: Vec::new(),
        };

        for item in &ast.items {
            match item {
                Item::Worker(w) => {
                    if w.is_default {
                        exports.default = Some(w.name.to_string());
                    } else if w.is_exported {
                        exports.named.push(w.name.to_string());
                    }
                }
                Item::Trait(t) => {
                    if t.is_default {
                        exports.default = Some(t.name.to_string());
                    } else if t.is_exported {
                        exports.named.push(t.name.to_string());
                    }
                }
                Item::Function(f) => {
                    if f.is_default {
                        exports.default = Some(f.name.to_string());
                    } else if f.is_exported {
                        exports.named.push(f.name.to_string());
                    }
                }
                _ => {}
            }
        }

        exports
    }

    /// Get compilation order (topologically sorted)
    pub fn compilation_order(&self) -> Vec<&Module> {
        let mut order = Vec::new();
        let mut visited = HashSet::new();

        // Start with modules that have no dependencies, then work up
        for module_id in self.modules.keys() {
            self.topological_visit(module_id, &mut visited, &mut order);
        }

        order
    }

    /// Recursive topological sort helper
    fn topological_visit<'a>(
        &'a self,
        module_id: &str,
        visited: &mut HashSet<String>,
        order: &mut Vec<&'a Module>,
    ) {
        if visited.contains(module_id) {
            return;
        }

        visited.insert(module_id.to_string());

        // Visit dependencies first
        if let Some(module) = self.modules.get(module_id) {
            for dep_id in &module.dependencies {
                self.topological_visit(dep_id, visited, order);
            }
            order.push(module);
        }
    }

    /// Get a module by ID
    pub fn get_module(&self, module_id: &str) -> Option<&Module> {
        self.modules.get(module_id)
    }

    /// Get exports for a module
    pub fn get_exports(&self, module_id: &str) -> Option<&ModuleExports> {
        self.exports.get(module_id)
    }

    /// Get all resolved modules
    pub fn modules(&self) -> &HashMap<String, Module> {
        &self.modules
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_to_module_id() {
        let resolver = ModuleResolver::new("/project");
        // Note: This test would need actual file system setup to work properly
        // For now, just testing the structure
    }
}
