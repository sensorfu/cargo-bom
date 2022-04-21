use std::collections::BTreeSet;
use std::fmt;
use std::io;
use std::io::prelude::*;
use std::path;
use std::str;

use anyhow::Result;
use cargo::core::dependency::DepKind;
use cargo::core::package::PackageSet;
use cargo::core::{Package, Resolve, Workspace};
use cargo::ops;
use cargo::util::Config;
use cargo::CargoResult;
use structopt::StructOpt;
use tabled::Tabled;

#[derive(StructOpt)]
#[structopt(bin_name = "cargo")]
enum Opts {
    #[structopt(name = "bom")]
    /// Display a Bill-of-Materials for Rust project
    Bom(Args),
}

#[derive(StructOpt)]
struct Args {
    /// List all dependencies instead of only top level ones
    #[structopt(long = "all", short = "a")]
    all: bool,
    /// Directory for all generated artifacts
    #[structopt(long = "target-dir", value_name = "DIRECTORY", parse(from_os_str))]
    target_dir: Option<path::PathBuf>,
    #[structopt(long = "manifest-path", value_name = "PATH", parse(from_os_str))]
    /// Path to Cargo.toml
    manifest_path: Option<path::PathBuf>,
    #[structopt(long = "verbose", short = "v", parse(from_occurrences))]
    /// Use verbose output (-vv very verbose/build.rs output)
    verbose: u32,
    #[structopt(long = "quiet", short = "q")]
    /// No output printed to stdout other than the tree
    quiet: bool,
    #[structopt(long = "color", value_name = "WHEN")]
    /// Coloring: auto, always, never
    color: Option<String>,
    #[structopt(long = "frozen")]
    /// Require Cargo.lock and cache are up to date
    frozen: bool,
    #[structopt(long = "locked")]
    /// Require Cargo.lock is up to date
    locked: bool,
    #[structopt(long = "offline")]
    /// Run without accessing the network
    offline: bool,
    #[structopt(short = "Z", value_name = "FLAG")]
    /// Unstable (nightly-only) flags to Cargo
    unstable_flags: Vec<String>,
}

#[derive(Debug, Tabled, PartialEq, Eq, PartialOrd, Ord)]
struct DepTable {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Version")]
    version: String,
    #[tabled(rename = "Licenses")]
    licenses: String,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct LicenseTable {
    name: String,
    version: String,
    license_files: BTreeSet<path::PathBuf>,
}

fn main() -> Result<()> {
    let mut config = Config::default()?;
    let Opts::Bom(args) = Opts::from_args();
    real_main(&mut config, args)
}

fn real_main(config: &mut Config, args: Args) -> Result<()> {
    config.configure(
        args.verbose,
        args.quiet,
        args.color.as_deref(),
        args.frozen,
        args.locked,
        args.offline,
        &args.target_dir,
        &args.unstable_flags,
        &[],
    )?;

    let manifest = args
        .manifest_path
        .unwrap_or_else(|| config.cwd().join("Cargo.toml"));
    let ws = Workspace::new(&manifest, config)?;
    let members: Vec<Package> = ws.members().cloned().collect();
    let (package_ids, resolve) = ops::resolve_ws(&ws)?;

    let dependencies = if args.all {
        all_dependencies(&members, package_ids, resolve)?
    } else {
        top_level_dependencies(&members, package_ids)?
    };

    let mut depencies_list = BTreeSet::new();
    let mut licenses_list = BTreeSet::new();

    for package in &dependencies {
        let name = package.name().to_string();
        let version = format!("{}", package.version());
        let licenses = format!("{}", package_licenses(package));
        let license_files = package_license_files(package)?;
        depencies_list.insert(DepTable {
            name: name.clone(),
            version: version.clone(),
            licenses,
        });
        licenses_list.insert(LicenseTable {
            name,
            version,
            license_files,
        });
    }

    fn make_table(list: BTreeSet<DepTable>) -> String {
        use tabled::{Style, Table};
        Table::new(list).with(Style::modern()).to_string()
    }

    let table = make_table(depencies_list);

    let stdout = io::stdout();
    let mut out = stdout.lock();

    out.write_all(table.as_bytes())?;
    out.flush()?;

    for LicenseTable {
        name,
        version,
        license_files,
    } in licenses_list
    {
        if license_files.is_empty() {
            continue;
        }

        writeln!(out, "\n-----BEGIN {} {} LICENSES-----", name, version)?;

        let mut licenses_to_print = license_files.len();
        for file in license_files {
            let buf = std::fs::read(file)?;
            out.write_all(&buf)?;
            if licenses_to_print > 1 {
                out.write_all(b"\n-----NEXT LICENSE-----\n")?;
                licenses_to_print -= 1;
            }
        }

        writeln!(out, "\n-----END {} {} LICENSES-----", name, version)?;
        out.flush()?;
    }

    Ok(())
}

fn top_level_dependencies(
    members: &[Package],
    package_ids: PackageSet<'_>,
) -> CargoResult<BTreeSet<Package>> {
    let mut dependencies = BTreeSet::new();

    for member in members {
        for dependency in member.dependencies() {
            // Filter out Build and Development dependencies
            match dependency.kind() {
                DepKind::Normal => (),
                DepKind::Build | DepKind::Development => continue,
            }
            if let Some(dep) = package_ids
                .package_ids()
                .find(|id| dependency.matches_id(*id))
            {
                let package = package_ids.get_one(dep)?;
                dependencies.insert(package.to_owned());
            }
        }
    }

    // Filter out our own workspace crates from dependency list
    for member in members {
        dependencies.remove(member);
    }

    Ok(dependencies)
}

fn all_dependencies(
    members: &[Package],
    package_ids: PackageSet<'_>,
    resolve: Resolve,
) -> CargoResult<BTreeSet<Package>> {
    let mut dependencies = BTreeSet::new();

    for package_id in resolve.iter() {
        let package = package_ids.get_one(package_id)?;
        if members.contains(package) {
            // Skip listing our own packages in our workspace
            continue;
        }
        dependencies.insert(package.to_owned());
    }

    Ok(dependencies)
}

fn package_licenses(package: &Package) -> Licenses<'_> {
    let metadata = package.manifest().metadata();

    if let Some(ref license_str) = metadata.license {
        let licenses: BTreeSet<&str> = license_str
            .split("OR")
            .flat_map(|s| s.split("AND"))
            .flat_map(|s| s.split('/'))
            .map(str::trim)
            .collect();
        return Licenses::List(licenses);
    }

    if let Some(ref license_file) = metadata.license_file {
        return Licenses::File(license_file);
    }

    Licenses::Missing
}

static LICENCE_FILE_NAMES: &[&str] = &["LICENSE", "UNLICENSE", "COPYRIGHT"];

fn package_license_files(package: &Package) -> io::Result<BTreeSet<path::PathBuf>> {
    let mut result = BTreeSet::new();

    let path = package
        .manifest_path()
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Package manifest path missing"))?;

    let metadata = package.manifest().metadata();
    if let Some(ref license_file) = metadata.license_file {
        let file = path.join(license_file);
        if file.exists() {
            result.insert(file);
        }
    }

    for entry in path.read_dir()?.flatten() {
        if let Ok(name) = entry.file_name().into_string() {
            for license_name in LICENCE_FILE_NAMES {
                if name.starts_with(license_name) {
                    result.insert(entry.path());
                }
            }
        }
    }

    Ok(result)
}

#[derive(Debug)]
enum Licenses<'a> {
    // Use BTreeSet to get alphabetical order automatically.
    List(BTreeSet<&'a str>),
    File(&'a str),
    Missing,
}

use itertools::Itertools;

impl<'a> fmt::Display for Licenses<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match *self {
            Licenses::File(_) => write!(f, "Specified in license file"),
            Licenses::Missing => write!(f, "Missing"),
            Licenses::List(ref lic_names) => {
                let lics = lic_names.iter().map(ToString::to_string).join(", ");
                write!(f, "{}", lics)
            }
        }
    }
}
