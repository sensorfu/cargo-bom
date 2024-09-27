use std::collections::BTreeSet;
use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;

use cargo_metadata::{camino, DependencyKind};
use itertools::Itertools;
use tabled::Tabled;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    bom: Option<BomCli>,
}

#[derive(Debug, Subcommand)]
enum BomCli {
    Bom {
        /// Path to Cargo.toml
        #[arg(long)]
        manifest_path: Option<PathBuf>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut cmd = cargo_metadata::MetadataCommand::new();

    if let Some(bom) = cli.bom {
        match bom {
            BomCli::Bom { manifest_path } => {
                if let Some(path) = manifest_path {
                    cmd.manifest_path(path);
                }
            }
        }
    }

    let metadata = cmd.exec()?;

    let mut depencies_list = BTreeSet::new();
    let mut licenses_list = BTreeSet::new();

    let members = metadata.workspace_packages();

    for member in &members {
        for dependency in &member.dependencies {
            // We only care about normal dependencies
            if dependency.kind != DependencyKind::Normal {
                continue;
            }

            if let Some(dep) = metadata.packages.iter().find(|p| p.name == dependency.name) {
                // Skip crates in repository
                if members.iter().any(|m| m.name == dep.name) {
                    continue;
                }

                let name = dep.name.clone();
                let version = dep.version.to_string();
                let licenses = package_licenses(dep).to_string();
                let license_files = package_license_files(dep)?;

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
        }
    }

    fn make_table(list: BTreeSet<DepTable>) -> String {
        use tabled::settings::{Settings, Style};
        use tabled::Table;
        let config = Settings::empty().with(Style::modern());
        Table::new(list).with(config).to_string()
    }

    let table = make_table(depencies_list);

    let mut out = io::stdout().lock();

    out.write_all(table.as_bytes())?;
    out.write_all(b"\n")?;
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

        writeln!(out, "\n-----BEGIN {name} {version} LICENSES-----")?;

        let mut licenses_to_print = license_files.len();
        for file in license_files {
            let buf = std::fs::read(file)?;
            out.write_all(&buf)?;
            if licenses_to_print > 1 {
                out.write_all(b"\n-----NEXT LICENSE-----\n")?;
                licenses_to_print -= 1;
            }
        }

        writeln!(out, "\n-----END {name} {version} LICENSES-----")?;
        out.flush()?;
    }

    Ok(())
}

static LICENCE_FILE_NAMES: &[&str] = &["LICENSE", "UNLICENSE", "COPYRIGHT"];

#[derive(Debug, Tabled, PartialEq, Eq, PartialOrd, Ord)]
struct DepTable {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Version")]
    version: String,
    #[tabled(rename = "Licenses")]
    licenses: String,
}

#[derive(Debug)]
enum Licenses<'a> {
    // Use BTreeSet to get alphabetical order automatically.
    List(BTreeSet<&'a str>),
    File(String),
    Missing,
}

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

fn package_licenses(package: &cargo_metadata::Package) -> Licenses<'_> {
    if let Some(ref license_str) = package.license {
        let licenses: BTreeSet<&str> = license_str
            .split("OR")
            .flat_map(|s| s.split("AND"))
            .flat_map(|s| s.split('/'))
            .map(str::trim)
            .collect();
        return Licenses::List(licenses);
    }

    if let Some(ref license_file) = package.license_file() {
        return Licenses::File(license_file.to_string());
    }

    Licenses::Missing
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct LicenseTable {
    name: String,
    version: String,
    license_files: BTreeSet<camino::Utf8PathBuf>,
}

pub fn package_license_files(
    package: &cargo_metadata::Package,
) -> io::Result<BTreeSet<camino::Utf8PathBuf>> {
    let mut result = BTreeSet::new();

    let path = package
        .manifest_path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Package manifest path missing"))?;

    if let Some(ref license_file) = package.license_file() {
        let file = path.join(license_file);
        if file.exists() {
            result.insert(file);
        }
    }

    for entry in path.read_dir()?.flatten() {
        if let Ok(name) = entry.file_name().into_string() {
            for license_name in LICENCE_FILE_NAMES {
                if name.starts_with(license_name) {
                    match camino::Utf8PathBuf::from_path_buf(entry.path()) {
                        Ok(path) => {
                            result.insert(path);
                        }
                        Err(err) => panic!("Invalid path: {err:?}"),
                    }
                }
            }
        }
    }

    Ok(result)
}
