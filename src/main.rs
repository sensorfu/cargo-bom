use cargo::core::{Package, Workspace};
use cargo::ops;
use cargo::util::Config;

use std::collections::BTreeSet;
use std::fmt;
use std::io::prelude::*;
use std::io;
use std::path;
use std::str;

#[derive(Debug)]
enum Licenses<'a> {
    Licenses(BTreeSet<&'a str>),
    File(&'a str),
    Missing,
}

impl<'a> fmt::Display for Licenses<'a> {
    fn fmt(self: &Self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match *self {
            Licenses::File(_) => write!(f, "Specified in license file"),
            Licenses::Missing => write!(f, "Missing"),
            Licenses::Licenses(ref lic_names) => {
                let lics: Vec<String> = lic_names.iter().map(|s| String::from(*s)).collect();
                write!(f, "{}", lics.join(", "))
            }
        }
    }
}

fn main() -> Result<(), Error> {
    let config = Config::default()?;
    let manifest = config.cwd().join("Cargo.toml");
    let ws = Workspace::new(&manifest, &config)?;
    let members: Vec<&Package> = ws.members().collect();
    let (package_ids, resolve) = ops::resolve_ws(&ws)?;

    let mut packages = Vec::new();
    for package_id in resolve.iter() {
        let package = package_ids.get_one(package_id)?;
        if members.contains(&package) {
            // Skip listing our own packages in our workspace
            continue;
        }
        let name = package.name().to_owned();
        let version = format!("{}", package.version());
        let licenses = format!("{}", package_licenses(package));
        let license_files = package_license_files(package)?;
        packages.push((name, version, licenses, license_files));
    }

    packages.sort();

    let stdout = io::stdout();
    let mut out = stdout.lock();

    {
        let mut tw = tabwriter::TabWriter::new(&mut out);
        writeln!(tw, "Name\t| Version\t| Licenses")?;
        writeln!(tw, "----\t| -------\t| --------")?;
        for (name, version, licenses, _) in &packages {
            writeln!(tw, "{}\t| {}\t| {}", &name, &version, &licenses)?;
        }

        // TabWriter flush() makes the actual write to stdout.
        tw.flush()?;
    }

    println!();

    for (name, version, _, license_files) in packages {
        if license_files.is_empty() {
            continue;
        }

        println!("-----BEGIN {} {} LICENSES-----", name, version);

        let mut buf = Vec::new();
        for file in license_files {
            let mut fs = std::fs::File::open(file)?;
            fs.read_to_end(&mut buf)?;
            out.write_all(&buf)?;
            buf.clear();
        }

        println!("-----END {} {} LICENSES-----", name, version);
        println!();
    }

    Ok(())
}

fn package_licenses(package: &Package) -> Licenses<'_> {
    let metadata = package.manifest().metadata();

    if let Some(ref license_str) = metadata.license {
        let licenses: BTreeSet<&str> = license_str
            .split('/')
            .map(str::trim)
            .collect();
        return Licenses::Licenses(licenses);
    }

    if let Some(ref license_file) = metadata.license_file {
        return Licenses::File(license_file);
    }

    Licenses::Missing
}

fn package_license_files(package: &Package) -> io::Result<Vec<path::PathBuf>> {
    let mut result = Vec::new();

    if let Some(path) = package.manifest_path().parent() {
        for entry in path.read_dir()? {
            if let Ok(entry) = entry {
                if let Ok(name) = entry.file_name().into_string() {
                    if name.starts_with("LICENSE") {
                        result.push(entry.path())
                    }
                }
            }
        }
    }

    Ok(result)
}

#[derive(Debug)]
struct Error;

impl From<failure::Error> for Error {
    fn from(err: failure::Error) -> Self {
        cargo_exit(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        let failure = failure::Error::from_boxed_compat(Box::new(err));
        cargo_exit(failure)
    }
}

fn cargo_exit<E: Into<cargo::CliError>>(err: E) -> ! {
    let mut shell = cargo::core::shell::Shell::new();
    cargo::exit_with_error(err.into(), &mut shell)
}
