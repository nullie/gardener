use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use crate::{
    config::{Config, OwnerModule},
    declarative::{DeclaredFileType, tmpfiles::add_systemd_tmpfiles, tree::Tree},
    fs::{FileType, Visitor, visit_dirs},
};

pub fn check_untracked() -> eyre::Result<()> {
    let config = Config::load()?;

    let mut tree = Tree::new();

    add_systemd_tmpfiles(&mut tree)?;

    config.add_to_tree(&mut tree)?;

    let mut visitor = SimpleVisitor::default();

    visit_dirs(Path::new("/"), &tree, &mut visitor)?;

    visitor.print_report();

    Ok(())
}

pub fn suggest_config() -> eyre::Result<()> {
    let config = Config::load()?;

    let mut tree = Tree::new();

    add_systemd_tmpfiles(&mut tree)?;

    config.add_to_tree(&mut tree)?;

    let mut visitor = SimpleVisitor::default();

    crate::fs::visit_dirs(Path::new("/"), &tree, &mut visitor)?;

    visitor.print_suggested_config();

    Ok(())
}

pub fn print_untracked() -> eyre::Result<()> {
    let config = Config::load()?;

    let mut tree = Tree::new();

    add_systemd_tmpfiles(&mut tree)?;

    config.add_to_tree(&mut tree)?;

    let mut visitor = SimpleVisitor::default();

    crate::fs::visit_dirs(Path::new("/"), &tree, &mut visitor)?;

    visitor.print_untracked();

    Ok(())
}

struct UntrackedPath {
    path: PathBuf,
    file_type: FileType,
}

impl std::fmt::Display for UntrackedPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_symbol = match self.file_type {
            FileType::Directory => 'd',
            FileType::File(declared_type) => match declared_type {
                DeclaredFileType::Regular => 'f',
                DeclaredFileType::Symlink => 's',
                DeclaredFileType::Fifo => 'p',
                DeclaredFileType::CharDevice => 'b',
                DeclaredFileType::BlockDevice => 'l',
            },
            FileType::Other(_file_type) => '?',
        };

        write!(f, "{} {}", type_symbol, self.path.display())
    }
}

#[derive(Default)]
struct SimpleVisitor<'a> {
    untracked: Vec<UntrackedPath>,
    tracked_by_disabled_module: BTreeMap<OwnerModule<'a>, Vec<UntrackedPath>>,
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
                OwnerModule::System { name, enabled } => {
                    assert!(!enabled);

                    system_modules.push(name);
                }
                OwnerModule::User {
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
        maybe_owner: Option<OwnerModule<'b>>,
        file_type: FileType,
    ) {
        if let Some(owner) = maybe_owner {
            self.tracked_by_disabled_module
                .entry(owner)
                .or_default()
                .push(UntrackedPath { path, file_type });
        } else {
            self.untracked.push(UntrackedPath { path, file_type });
        }
    }

    fn report_mismatching_path(
        &mut self,
        path: PathBuf,
        owner: Option<OwnerModule>,
        expected: FileType,
        found: FileType,
    ) {
        eprintln!(
            "{owner:?} {}: unexpected entry, expected {expected:?}, found {found:?}",
            path.display()
        );
    }
}

impl<'a> Visitor<'a> for SimpleVisitor<'a> {
    fn visit_file(
        &mut self,
        path: PathBuf,
        maybe_owner: Option<OwnerModule<'a>>,
        file_type: FileType,
        maybe_expected: Option<FileType>,
    ) {
        if let Some(expected) = maybe_expected {
            if expected != file_type {
                self.report_mismatching_path(path, maybe_owner, expected, file_type);
            } else if let Some(owner) = maybe_owner
                && !owner.enabled()
            {
                self.report_untracked_path(path, Some(owner), file_type);
            }
        } else {
            self.report_untracked_path(path, maybe_owner, file_type);
        }
    }

    fn visit_error(&mut self, dir: PathBuf, e: std::io::Error) {
        eprintln!("Failed to read directory {:?}: {}", dir, e);
    }

    fn visit_dir(
        &mut self,
        path: PathBuf,
        maybe_owner: Option<OwnerModule<'a>>,
        maybe_expected: Option<FileType>,
        expected_children: bool,
    ) -> bool {
        if let Some(expected) = maybe_expected {
            if expected == FileType::Directory {
                expected_children
            } else {
                self.report_mismatching_path(path, maybe_owner, expected, FileType::Directory);
                false
            }
        } else {
            self.report_untracked_path(path, maybe_owner, FileType::Directory);
            false
        }
    }
}
