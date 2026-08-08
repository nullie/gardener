use std::{
    collections::BTreeMap,
    ops::ControlFlow,
    path::{Path, PathBuf},
};

use rootcause::Result;

use crate::{
    config::Config,
    declarative::{self, Owner, PathType, Properties},
    fs::{self, unix},
    presentation::UntrackedPath,
};

pub fn check_untracked() -> Result<()> {
    let config = Config::load()?;
    let tree = config.to_tree()?;

    let mut visitor = UntrackedVisitor::default();

    unix::walker::walk_tree(Path::new("/"), &tree, &mut visitor)?;

    visitor.print_report();

    Ok(())
}

pub fn suggest_config() -> Result<()> {
    let config = Config::load()?;
    let tree = config.to_tree()?;

    let mut visitor = UntrackedVisitor::default();

    unix::walker::walk_tree(Path::new("/"), &tree, &mut visitor)?;

    visitor.print_suggested_config();

    Ok(())
}

pub fn print_untracked() -> Result<()> {
    let config = Config::load()?;
    let tree = config.to_tree()?;

    let mut visitor = UntrackedVisitor::default();

    unix::walker::walk_tree(Path::new("/"), &tree, &mut visitor)?;

    visitor.print_untracked();

    Ok(())
}

#[derive(Default)]
struct UntrackedVisitor<'a> {
    untracked: Vec<UntrackedPath<PathBuf>>,
    tracked_by_disabled_module: BTreeMap<Owner<'a>, Vec<UntrackedPath<PathBuf>>>,
}

impl<'a> UntrackedVisitor<'a> {
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
                Owner::System { name, enabled } => {
                    assert!(!enabled);

                    system_modules.push(name);
                }
                Owner::User {
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

    fn report_untracked_path<'b, 'c>(
        &'b mut self,
        path: &'c Path,
        maybe_properties: Option<Properties<'a>>,
        path_type: unix::PathType,
    ) {
        if let Some(properties) = maybe_properties {
            self.tracked_by_disabled_module
                .entry(properties.owner)
                .or_default()
                .push(UntrackedPath {
                    path: path.to_owned(),
                    path_type,
                });
        } else {
            self.untracked.push(UntrackedPath {
                path: path.to_owned(),
                path_type,
            });
        }
    }

    fn report_mismatching_path(
        &mut self,
        path: &Path,
        maybe_properties: Option<Properties<'_>>,
        expected: unix::PathType,
        found: unix::PathType,
    ) {
        let maybe_owner = maybe_properties.map(|properties| properties.owner);

        eprintln!(
            "{maybe_owner:?} {}: unexpected entry, expected {expected:?}, found {found:?}",
            path.display()
        );
    }

    fn check_path<'b>(
        &'b mut self,
        path: &Path,
        path_type: unix::PathType,
        declared: declarative::Entry<Properties<'a>>,
    ) -> ControlFlow<(), ()> {
        if let Some(declared_path_type) = declared.maybe_path_type {
            let expected_path_type = unix::PathType::from_declarative_path_type(declared_path_type);

            if expected_path_type != path_type {
                self.report_mismatching_path(
                    path,
                    declared.maybe_properties,
                    expected_path_type,
                    path_type,
                );

                return ControlFlow::Break(());
            }
        }

        if declared.maybe_path_type.is_none() {
            self.report_untracked_path(path, declared.maybe_properties, path_type);

            return ControlFlow::Break(());
        }

        if declared
            .maybe_properties
            .is_some_and(|properties| !properties.owner.enabled())
        {
            self.report_untracked_path(path, declared.maybe_properties, path_type);

            return ControlFlow::Break(());
        }

        ControlFlow::Continue(())
    }
}

impl<'a> unix::walker::Visitor<Properties<'a>> for UntrackedVisitor<'a> {
    fn visit_dir(
        &mut self,
        path: &Path,
        declared: declarative::Entry<Properties<'a>>,
        _has_declared_children: bool,
    ) -> ControlFlow<(), ()> {
        self.check_path(path, unix::PathType::Directory(()), declared)?;

        if let Some(declared_path_type) = declared.maybe_path_type {
            match declared_path_type {
                PathType::Directory(declarative::DirectoryProperties { owns_contents }) => {
                    if let Some(properties) = declared.maybe_properties {
                        // If not enabled, then it's untracked
                        if !properties.owner.enabled() {
                            self.report_untracked_path(
                                path,
                                declared.maybe_properties,
                                unix::PathType::Directory(()),
                            );

                            ControlFlow::Break(())
                        } else if !owns_contents {
                            // Recurse to discover untracked files
                            ControlFlow::Continue(())
                        } else {
                            // Owns everything, everything is tracked
                            ControlFlow::Break(())
                        }
                    } else {
                        ControlFlow::Continue(())
                    }
                }
                PathType::File(_) => {
                    self.report_mismatching_path(
                        path,
                        declared.maybe_properties,
                        unix::PathType::from_declarative_path_type(declared_path_type),
                        unix::PathType::Directory(()),
                    );

                    ControlFlow::Break(())
                }
            }
        } else {
            self.report_untracked_path(path, None, unix::PathType::Directory(()));
            ControlFlow::Break(())
        }
    }

    fn visit_file(
        &mut self,
        path: &Path,
        file_type: unix::FileType,
        declared: declarative::Entry<Properties<'a>>,
        _len: u64,
    ) {
        if let Some(declared_path_type) = declared.maybe_path_type {
            let expected_path_type = unix::PathType::from_declarative_path_type(declared_path_type);
            let path_type = fs::Entry::File(file_type);

            if expected_path_type != path_type {
                self.report_mismatching_path(
                    path,
                    declared.maybe_properties,
                    expected_path_type,
                    path_type,
                );
            }
        }

        if declared
            .maybe_properties
            .is_none_or(|properties| !properties.owner.enabled())
        {
            self.report_untracked_path(
                path,
                declared.maybe_properties,
                unix::PathType::File(file_type),
            );
        }
    }

    fn visit_error(&mut self, dir: &Path, e: std::io::Error) {
        eprintln!("Failed to read directory {:?}: {}", dir, e);
    }
}
