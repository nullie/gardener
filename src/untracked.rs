use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use crate::{config::Config, declarative, fs, presentation::UntrackedPath};

pub fn check_untracked() -> eyre::Result<()> {
    let config = Config::load()?;
    let tree = config.to_tree()?;

    let mut visitor = SimpleVisitor::default();

    fs::visit_dirs(Path::new("/"), &tree, &mut visitor)?;

    visitor.print_report();

    Ok(())
}

pub fn suggest_config() -> eyre::Result<()> {
    let config = Config::load()?;
    let tree = config.to_tree()?;

    let mut visitor = SimpleVisitor::default();

    fs::visit_dirs(Path::new("/"), &tree, &mut visitor)?;

    visitor.print_suggested_config();

    Ok(())
}

pub fn print_untracked() -> eyre::Result<()> {
    let config = Config::load()?;
    let tree = config.to_tree()?;

    let mut visitor = SimpleVisitor::default();

    fs::visit_dirs(Path::new("/"), &tree, &mut visitor)?;

    visitor.print_untracked();

    Ok(())
}

#[derive(Default)]
struct SimpleVisitor<'a> {
    untracked: Vec<UntrackedPath>,
    tracked_by_disabled_module: BTreeMap<declarative::Owner<'a>, Vec<UntrackedPath>>,
}

impl<'a> SimpleVisitor<'a> {
    fn print_untracked(&self) {
        for untracked_path in &self.untracked {
            println!("{}", untracked_path.path.display());
        }

        for tracked_paths in self.tracked_by_disabled_module.values() {
            for tracked_path in tracked_paths {
                println!("{}", tracked_path.path.display());
            }
        }
    }

    fn print_report(&self) {
        if !self.untracked.is_empty() {
            println!("Untracked paths:");

            for untracked_path in &self.untracked {
                println!("  {}", untracked_path);
            }

            if !self.tracked_by_disabled_module.is_empty() {
                println!();
            }
        }

        if !self.tracked_by_disabled_module.is_empty() {
            println!("Tracked by disabled modules:");

            for (owner, tracked_paths) in self.tracked_by_disabled_module.iter() {
                println!("  {:?}", owner);

                for tracked_path in tracked_paths {
                    println!("    {}", tracked_path);
                }
            }
        }
    }

    fn print_suggested_config(&self) {
        let mut system_modules = Vec::new();
        let mut user_modules: BTreeMap<&str, Vec<_>> = BTreeMap::new();

        for owner in self.tracked_by_disabled_module.keys() {
            match owner {
                declarative::Owner::System { name, enabled } => {
                    assert!(!enabled);

                    system_modules.push(name);
                }
                declarative::Owner::User {
                    name,
                    user,
                    enabled,
                } => {
                    assert!(!enabled);

                    user_modules.entry(user).or_default().push(name);
                }
                _ => panic!("TODO: refactor types, adhoc modules should not be here"),
            };
        }

        println!("services.gardener = {{");
        println!("  enabledModules = {{");

        for name in system_modules {
            println!("    {name} = true;")
        }

        println!("  }};");

        println!("  users = {{");

        for (user, modules) in user_modules {
            println!("    {user} = {{");
            println!("      modules = {{");

            for module in modules {
                println!("        {module} = true;");
            }

            println!("      }};");
            println!("    }};");
        }

        println!("  }};");

        println!("}};");
    }

    fn report_untracked_path<'b: 'a>(
        &mut self,
        path: PathBuf,
        maybe_properties: Option<declarative::Properties<'b>>,
        file_type: fs::PathType,
    ) {
        if let Some(properties) = maybe_properties {
            self.tracked_by_disabled_module
                .entry(properties.owner)
                .or_default()
                .push(UntrackedPath { path, file_type });
        } else {
            self.untracked.push(UntrackedPath { path, file_type });
        }
    }

    fn report_mismatching_path(
        &mut self,
        path: PathBuf,
        maybe_properties: Option<declarative::Properties<'_>>,
        expected: fs::PathType,
        found: fs::PathType,
    ) {
        let maybe_owner = maybe_properties.map(|properties| properties.owner);

        eprintln!(
            "{maybe_owner:?} {}: unexpected entry, expected {expected:?}, found {found:?}",
            path.display()
        );
    }
}

impl<'a> fs::Visitor<'a> for SimpleVisitor<'a> {
    fn visit_file(
        &mut self,
        path: PathBuf,
        file_type: fs::PathType,
        _len: u64,
        maybe_expected: Option<fs::PathType>,
        maybe_properties: Option<crate::declarative::Properties<'a>>,
    ) {
        if let Some(expected) = maybe_expected {
            if expected != file_type {
                self.report_mismatching_path(path, maybe_properties, expected, file_type);
            } else if let Some(properties) = maybe_properties
                && !properties.owner.enabled()
            {
                self.report_untracked_path(path, Some(properties), file_type);
            }
        } else {
            self.report_untracked_path(path, maybe_properties, file_type);
        }
    }

    fn visit_error(&mut self, dir: PathBuf, e: std::io::Error) {
        eprintln!("Failed to read directory {:?}: {}", dir, e);
    }

    fn visit_dir(
        &mut self,
        path: PathBuf,
        maybe_expected_path_type: Option<fs::PathType>,
        maybe_properties: Option<crate::declarative::Properties<'a>>,
        has_declared_children: bool,
    ) -> bool {
        if let Some(expected_path_type) = maybe_expected_path_type {
            if expected_path_type == fs::PathType::Directory {
                has_declared_children
            } else {
                self.report_mismatching_path(
                    path,
                    maybe_properties,
                    expected_path_type,
                    fs::PathType::Directory,
                );
                false
            }
        } else {
            self.report_untracked_path(path, maybe_properties, fs::PathType::Directory);
            false
        }
    }
}
