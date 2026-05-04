use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use systemd_tmpfiles::Directive;

use crate::declarative::{
    DeclaredPathType,
    tree::{ClosedNodeType, Node, Tree},
};
use crate::{
    config::{Config, OwnerModule},
    declarative::DeclaredFileType,
};

pub fn check_untracked() -> eyre::Result<()> {
    let config = Config::load()?;

    let mut tree = Tree::new();

    add_systemd_tmpfiles(&mut tree)?;

    config.add_to_tree(&mut tree)?;

    let mut visitor = SimpleVisitor::default();

    visit_dirs(Path::new("/"), &tree.root, &mut visitor)?;

    visitor.print_report();

    Ok(())
}

pub fn suggest_config() -> eyre::Result<()> {
    let config = Config::load()?;

    let mut tree = Tree::new();

    add_systemd_tmpfiles(&mut tree)?;

    config.add_to_tree(&mut tree)?;

    let mut visitor = SimpleVisitor::default();

    visit_dirs(Path::new("/"), &tree.root, &mut visitor)?;

    visitor.print_suggested_config();

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

trait Visitor<'a> {
    fn visit_untracked_path(
        &mut self,
        path: PathBuf,
        owner: Option<OwnerModule<'a>>,
        file_type: FileType,
    );
    fn visit_mismatching_path(
        &mut self,
        path: PathBuf,
        owner: Option<OwnerModule>,
        expected: FileType,
        found: FileType,
    );
    fn visit_error(&mut self, dir: PathBuf, e: std::io::Error);
}

#[derive(Default)]
struct SimpleVisitor<'a> {
    untracked: Vec<UntrackedPath>,
    tracked_by_disabled_module: BTreeMap<OwnerModule<'a>, Vec<UntrackedPath>>,
}

impl SimpleVisitor<'_> {
    fn print_report(&self) {
        println!("Untracked paths:");

        for untracked_path in &self.untracked {
            println!("  {}", untracked_path);
        }

        println!();

        println!("Tracked by disabled modules:");

        for (owner, tracked_paths) in self.tracked_by_disabled_module.iter() {
            println!("  {:?}", owner);

            for tracked_path in tracked_paths {
                println!("    {}", tracked_path);
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

        println!("gardener.config = {{");
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
}

impl<'a> Visitor<'a> for SimpleVisitor<'a> {
    fn visit_untracked_path(
        &mut self,
        path: PathBuf,
        maybe_owner: Option<OwnerModule<'a>>,
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

    fn visit_mismatching_path(
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

    fn visit_error(&mut self, dir: PathBuf, e: std::io::Error) {
        eprintln!("Failed to read directory {:?}: {}", dir, e);
    }
}

fn visit_dirs<'a>(
    dir: &Path,
    tree_directory: &'a BTreeMap<OsString, Node>,
    visitor: &mut impl Visitor<'a>,
) -> eyre::Result<()> {
    match fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                let file_type = FileType::new(entry.file_type()?);
                let maybe_tree_node = tree_directory.get(entry.file_name().as_os_str());

                match (maybe_tree_node, file_type) {
                    (Some(Node::Open(_maybe_owner, children)), FileType::Directory) => {
                        visit_dirs(&path, children, visitor)?;
                    }
                    (Some(tree_node), file_type) => {
                        let maybe_owner = match tree_node {
                            Node::Open(maybe_owner, _) => *maybe_owner,
                            Node::Closed(owner, _) => Some(*owner),
                        };

                        let expected = match tree_node {
                            Node::Open(_, _) => FileType::Directory,
                            Node::Closed(_, ClosedNodeType::ClosedDirectory) => FileType::Directory,
                            Node::Closed(_, ClosedNodeType::File(declared_file_type)) => {
                                FileType::File(*declared_file_type)
                            }
                        };

                        if expected != file_type {
                            visitor.visit_mismatching_path(path, maybe_owner, expected, file_type);
                        } else if let Some(owner) = maybe_owner
                            && !owner.enabled()
                        {
                            visitor.visit_untracked_path(path, Some(owner), file_type);
                        }
                    }
                    (None, file_type) => {
                        visitor.visit_untracked_path(path, None, file_type);
                    }
                }
            }
        }
        Err(e) => {
            visitor.visit_error(dir.to_owned(), e);
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum FileType {
    Directory,
    File(DeclaredFileType),
    Other(fs::FileType),
}

impl FileType {
    fn new(file_type: fs::FileType) -> Self {
        if file_type.is_dir() {
            FileType::Directory
        } else if file_type.is_file() {
            FileType::File(DeclaredFileType::Regular)
        } else if file_type.is_symlink() {
            FileType::File(DeclaredFileType::Symlink)
        } else {
            FileType::Other(file_type)
        }
    }
}

fn add_systemd_tmpfiles(tree: &mut Tree) -> eyre::Result<()> {
    let owner = OwnerModule::AdhocSystem {
        name: "systemd-tmpfiles",
    };
    let output = std::process::Command::new("systemd-tmpfiles")
        .arg("--cat-config")
        .output()?;

    // FIXME: return error
    assert!(output.status.success());

    let output = String::from_utf8(output.stdout)?;

    let parsed = systemd_tmpfiles::parser::parse_str(&output)?;

    for entry in parsed {
        let maybe_entry_type = match entry.directive() {
            Directive::CreateSymlink { .. } => {
                Some(DeclaredPathType::File(DeclaredFileType::Symlink))
            }
            Directive::CreateFile { .. } | Directive::WriteToFile { .. } => {
                Some(DeclaredPathType::File(DeclaredFileType::Regular))
            }
            Directive::CreateFifo { .. } => Some(DeclaredPathType::File(DeclaredFileType::Fifo)),
            Directive::CreateCharDeviceNode { .. } => {
                Some(DeclaredPathType::File(DeclaredFileType::CharDevice))
            }
            Directive::CreateBlockDeviceNode { .. } => {
                Some(DeclaredPathType::File(DeclaredFileType::BlockDevice))
            }
            Directive::CreateDirectory { .. } | Directive::CreateSubvolume { .. } => {
                Some(DeclaredPathType::OpenDirectory)
            }
            _ => None,
        };

        if let Some(entry_type) = maybe_entry_type {
            // FIXME: return error
            assert!(!entry.path_is_glob());

            tree.add_path(owner, Path::new(entry.path()), entry_type)?;
        }
    }

    Ok(())
}
