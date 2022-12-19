# Bill of Materials for Rust Crates

![Crates.io](https://img.shields.io/crates/v/cargo-bom.svg)

> A Software Bill of Materials (software BOM) is a list of components in a piece
> of software. Software vendors often create products by assembling open source
> and commercial software components. The software BOM describes the components
> in a product. It is analogous to a list of ingredients on food packaging.

Source: [Wikipedia](https://en.wikipedia.org/wiki/Software_bill_of_materials)

This tool (`cargo bom`) can be used to construct Bill of Materials for software
using [Cargo](http://doc.crates.io/) package manager.

The output of `cargo bom` has two sections. First it prints out a table with all
top level dependencies, version numbers and names of licenses. Then it prints
all license texts found from depended projects (files matching globs "LICENSE*"
and "UNLICENSE*").

## Example usage

```console
$ cargo bom >BOM.txt
$ head BOM.txt
┌───────────┬─────────┬─────────────────┐
│ Name      │ Version │ Licenses        │
├───────────┼─────────┼─────────────────┤
│ anyhow    │ 1.0.68  │ Apache-2.0, MIT │
├───────────┼─────────┼─────────────────┤
│ cargo     │ 0.67.0  │ Apache-2.0, MIT │
├───────────┼─────────┼─────────────────┤
│ itertools │ 0.10.5  │ Apache-2.0, MIT │
├───────────┼─────────┼─────────────────┤
│ structopt │ 0.3.26  │ Apache-2.0, MIT │

```

# Bill of Materials

The Bill of Materials for this project can be found from [BOM.txt](./BOM.txt).

# License

`cargo bom` is distributed under the terms of the MIT license.

See [LICENSE](./LICENSE) for details.
