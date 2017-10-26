# Bill of Materials for Rust Crates

> A Software Bill of Materials (software BOM) is a list of components in a piece
> of software. Software vendors often create products by assembling open source
> and commercial software components. The software BOM describes the components
> in a product. It is analogous to a list of ingredients on food packaging.

Source: [Wikipedia](https://en.wikipedia.org/wiki/Software_bill_of_materials)

This tool (`cargo bom`) can be used to construct Bill of Materials for software
using [Cargo](http://doc.crates.io/) package manager.

The output of `cargo bom` has two sections. First it prints out a table with all
dependencies, version numbers and names of licenses. Then it prints all license
texts found from depended projects (files matching glob "LICENSE*").

## Example usage

```console
$ cargo bom >BOM.txt
$ head BOM.txt 
Name                    | Version  | Licenses
advapi32-sys            | 0.2.0    | MIT
aho-corasick            | 0.6.3    | MIT, Unlicense
atty                    | 0.2.3    | MIT
backtrace               | 0.3.3    | Apache-2.0, MIT
backtrace-sys           | 0.1.16   | Apache-2.0, MIT
bitflags                | 0.7.0    | Apache-2.0, MIT
bitflags                | 0.9.1    | Apache-2.0, MIT
cargo                   | 0.22.0   | Apache-2.0, MIT
cc                      | 1.0.2    | Apache-2.0, MIT
```

# Bill of Materials

The Bill of Materials for this project can be found from [BOM.txt](./BOM.txt).

# License

`cargo bom` is distributed under the terms of the MIT license.

See [LICENSE](./LICENSE) for details.
