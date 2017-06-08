extern crate cargo;
extern crate rustc_serialize;

use cargo::core::Package;
use cargo::core::Workspace;
use cargo::ops;
use cargo::util::Config;

use std::env;
use std::fmt;
use std::path;
use std::io::Read;

#[derive(RustcDecodable)]
struct Options {
    flag_verbose: u32,
    flag_quiet: Option<bool>,
    flag_color: Option<String>,
    flag_frozen: bool,
    flag_locked: bool,
}

#[derive(Debug)]
enum Licenses {
    Licenses(Vec<String>),
    File(String),
    Missing,
}

impl fmt::Display for Licenses {
    fn fmt(self: &Self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            Licenses::File(_) => Ok(write!(f, "Specified in license file")?),
            Licenses::Missing => Ok(write!(f, "Missing")?),
            Licenses::Licenses(ref lic_names) => Ok(write!(f, "{}", lic_names.join(", "))?),
        }
    }
}

const USAGE_STR: &'static str = r#"
Produce Bill of Materials from Cargo project's depencies
Usage:
    cargo bom [options]
Options:
    -h, --help               Print this message
    -V, --version            Print version information
    -v, --verbose ...        Use verbose output    
    --frozen                 Require Cargo.lock and cache are up to date
    --locked                 Require Cargo.lock is up to date
    --color WHEN             Coloring: auto, always, never
This cargo subcommand will produce Bill of Materials (BOM) from crates the
project depends on.
"#;

fn main() {
    let config = Config::default().expect("cargo config");
    let args: Vec<String> = env::args().collect();
    
    let res = cargo::call_main_without_stdin(real_main, &config, USAGE_STR, &args, false);
    if let Err(e) = res {
        cargo::exit_with_error(e, &mut *config.shell());
    }    
}

fn real_main(options: Options, config: &Config) -> cargo::CliResult {
    config.configure(options.flag_verbose,
                     options.flag_quiet,
                     &options.flag_color,
                     options.flag_frozen,
                     options.flag_locked)?;

    let manifest = config.cwd().join("Cargo.toml");
    let ws = Workspace::new(&manifest, config)?;
    let (package_ids, resolve) = ops::resolve_ws(&ws)?;

    let mut packages: Vec<(String, String, String, Vec<path::PathBuf>)> = Vec::new();
    let mut longest_name = 0;
    for package_id in resolve.iter() {
        let package = package_ids.get(package_id)?;
        let name = package.name().to_owned();
        let version = format!("{}", package.version());
        let licenses = format!("{}", package_licenses(package));
        let license_files = package_license_files(package);
        longest_name = std::cmp::max(longest_name, name.len());
        packages.push((name, version, licenses, license_files));
    }

    packages.sort();

    right_pad_print(longest_name, "Name", "Version", "Licenses");
    println!("{:-<80}", "");

    for (name, version, licenses, _) in packages.clone() {
        right_pad_print(longest_name, &name, &version, &licenses);
    }

    println!("");

    for (name, version, _, license_files) in packages {
        if license_files.is_empty() {
            continue;
        }

        println!("-----BEGIN {} {} LICENSES-----", name, version);

        let mut buf = String::new();
        for file in license_files {
            let mut fs = std::fs::File::open(file).expect("File::open");
            fs.read_to_string(&mut buf).expect("read_to_string");
            println!("{}", buf);
            buf.clear();
        }

        println!("-----END {} {} LICENSES-----", name, version);
        println!("");
    }

    Ok(())
}

fn package_licenses(package: &Package) -> Licenses {
    let metadata = package.manifest().metadata();

    if let Some(ref license_str) = metadata.license {
        let mut licenses: Vec<String> = license_str
            .split('/')
            .map(|s| s.trim().to_owned())
            .collect();
        licenses.sort();
        licenses.dedup();
        return Licenses::Licenses(licenses);
    }

    if let Some(ref license_file) = metadata.license_file {
        return Licenses::File(license_file.to_owned());
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

fn right_pad_print(longest_name: usize, name: &str, version: &str, licenses: &str) {
    if longest_name <= 8 {
        println!("{:<8} | {:<8} | {}", name, version, licenses);
    } else if longest_name <= 16 {
        println!("{:<16} | {:<8} | {}", name, version, licenses);
    } else if longest_name <= 24 {
        println!("{:<24} | {:<8} | {}", name, version, licenses);
    } else {
        println!("{:<32} | {:<8} | {}", name, version, licenses);
    }
}
