use cargo::core::{Package, Workspace};
use cargo::ops;
use cargo::util::Config;

use std::collections::BTreeSet;
use std::fmt;
use std::io;
use std::io::prelude::*;
use std::path;

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

fn main() {
    let config = Config::default().expect("cargo config");
    let manifest = config.cwd().join("Cargo.toml");
    let ws = Workspace::new(&manifest, &config).expect("cargo workspace");
    let members: Vec<&Package> = ws.members().collect();
    let (package_ids, resolve) = ops::resolve_ws(&ws).expect("resolve workspace");

    let mut packages = Vec::new();
    for package_id in resolve.iter() {
        let package = package_ids.get_one(package_id).expect("get package ids");
        if members.contains(&package) {
            // Skip listing our own packages in our workspace
            continue;
        }
        let name = package.name().to_owned();
        let version = format!("{}", package.version());
        let licenses = format!("{}", package_licenses(package));
        let license_files = package_license_files(package);
        packages.push((name, version, licenses, license_files));
    }

    packages.sort();

    let stdout = io::stdout();
    let mut out = stdout.lock();

    {
        let mut tw = tabwriter::TabWriter::new(&mut out);
        writeln!(tw, "Name\t| Version\t| Licenses").expect("write");
        for (name, version, licenses, _) in packages.clone() {
            writeln!(tw, "{}\t| {}\t| {}", &name, &version, &licenses).expect("write");
        }
        tw.flush().expect("tw.flush"); // TabWriter flush() makes the actual write to stdout.
    }

    println!();

    for (name, version, _, license_files) in packages {
        if license_files.is_empty() {
            continue;
        }

        println!("-----BEGIN {} {} LICENSES-----", name, version);

        let mut buf = Vec::new();
        for file in license_files {
            let mut fs = std::fs::File::open(file).expect("File::open");
            fs.read_to_end(&mut buf).expect("read_to_end");
            out.write_all(&buf).expect("write_all");
            buf.clear();
        }

        println!("-----END {} {} LICENSES-----", name, version);
        println!();
    }
}

fn package_licenses(package: &Package) -> Licenses<'_> {
    let metadata = package.manifest().metadata();

    if let Some(ref license_str) = metadata.license {
        let licenses: BTreeSet<&str> = license_str
            .split('/')
            .map(|s| s.trim())
            .collect();
        return Licenses::Licenses(licenses);
    }

    if let Some(ref license_file) = metadata.license_file {
        return Licenses::File(license_file);
    }

    Licenses::Missing
}

fn package_license_files(package: &Package) -> Vec<path::PathBuf> {
    let mut result = Vec::new();

    if let Some(path) = package.manifest_path().parent() {
        for entry in path.read_dir().expect("read_dir call failed") {
            if let Ok(entry) = entry {
                if let Ok(name) = entry.file_name().into_string() {
                    if name.starts_with("LICENSE") {
                        result.push(entry.path())
                    }
                }
            }
        }
    }

    result
}
