# mailfmt

A simple and quick bidirectional converter between mbox and eml formats.

> [!NOTE]  
> This tool has been tested both ways, but it is not flawless. Always check the results to ensure that all of your data has been converted as expected.

## Installation

Ensure you have [Rust](https://www.rust-lang.org/tools/install) installed, then run 

```
cargo install --git https://codeberg.org/Blooym/mailfmt.git
```

## Usage

You can append the `--help` flag to see a full list of options for any command.

### EML to Mbox

Convert a directory of .eml files

```
mailfmt eml-to-mbox <INPUT_DIRECTORY> <OUTPUT_FILE>
```

### Mbox to EML

Convert a single .mbox file to a directory of .eml files

```
 mailfmt mbox-to-eml <INPUT_FILE> <OUTPUT_DIRECTORY>
```