# i18n-convert

[![CI](https://github.com/i18n-agent/i18n-convert/actions/workflows/ci.yml/badge.svg)](https://github.com/i18n-agent/i18n-convert/actions/workflows/ci.yml)
[![license](https://img.shields.io/github/license/i18n-agent/i18n-convert)](LICENSE)

Cross-platform localization file format converter — 32 formats, zero dependencies beyond the binary.

## The Problem

Every platform has its own localization format. Moving translations between Android XML, iOS Strings, XLIFF, PO, JSON, YAML, and 26 other formats means writing custom scripts, losing metadata, or paying for a SaaS tool.

## The Solution

One binary. Any format in, any format out. Lossless round-trips where possible, data loss warnings where not.

```bash
i18n-convert messages.json --to android-xml -o strings.xml
```

## Installation

### Download binary (recommended)

Download the latest release for your platform from [GitHub Releases](https://github.com/i18n-agent/i18n-convert/releases/latest).

**macOS (Apple Silicon):**
```bash
curl -L https://github.com/i18n-agent/i18n-convert/releases/latest/download/i18n-convert-aarch64-apple-darwin.tar.gz | tar xz
sudo mv i18n-convert /usr/local/bin/
```

**macOS (Intel):**
```bash
curl -L https://github.com/i18n-agent/i18n-convert/releases/latest/download/i18n-convert-x86_64-apple-darwin.tar.gz | tar xz
sudo mv i18n-convert /usr/local/bin/
```

**Linux (x64):**
```bash
curl -L https://github.com/i18n-agent/i18n-convert/releases/latest/download/i18n-convert-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv i18n-convert /usr/local/bin/
```

**Linux (ARM):**
```bash
curl -L https://github.com/i18n-agent/i18n-convert/releases/latest/download/i18n-convert-aarch64-unknown-linux-gnu.tar.gz | tar xz
sudo mv i18n-convert /usr/local/bin/
```

**Windows:**

Download `i18n-convert-x86_64-pc-windows-msvc.zip` from [Releases](https://github.com/i18n-agent/i18n-convert/releases/latest), extract, and add to your PATH.

### Build from source

```bash
git clone https://github.com/i18n-agent/i18n-convert.git
cd i18n-convert
cargo build --release
# Binary at target/release/i18n-convert
```

## Usage

### Basic conversion

```bash
# Auto-detect input format, convert to Android XML
i18n-convert en.json --to android-xml -o strings.xml

# Convert iOS strings to PO
i18n-convert Localizable.strings --to po -o messages.po

# Convert XLIFF to YAML
i18n-convert translations.xliff --to yaml-rails -o en.yml

# Output to stdout (pipe-friendly)
i18n-convert messages.po --to json
```

### Data loss warnings

When converting between formats with different capabilities, `i18n-convert` warns you before data is lost:

```bash
$ i18n-convert plurals.po --to csv
Data loss warnings:
  [ERROR] 3 entries use plurals (not supported by CSV)

Proceed? [y/N]
```

Use `--force` to skip prompts, or `--dry-run` to see warnings without writing.

### List all formats

```bash
i18n-convert --list-formats
```

### CLI flags

| Flag | Description |
|------|-------------|
| `--to <format>` | Target format (required) |
| `-o <file>` | Output file (default: stdout) |
| `--force` | Skip data loss confirmation |
| `--dry-run` | Show warnings without writing |
| `--verbose` | Show conversion details |
| `--list-formats` | List all supported formats |

## Supported Formats (32)

### Mobile & Desktop

| Format | ID | Extensions |
|--------|----|------------|
| Android XML | `android-xml` | `.xml` |
| Xcode String Catalog | `xcstrings` | `.xcstrings` |
| iOS Strings | `ios-strings` | `.strings` |
| iOS Stringsdict | `stringsdict` | `.stringsdict` |
| iOS Property List | `ios-plist` | `.plist` |
| Flutter ARB | `arb` | `.arb` |
| Qt Linguist | `qt` | `.ts` |

### Web & Frameworks

| Format | ID | Extensions |
|--------|----|------------|
| Structured JSON | `json` | `.json` |
| i18next JSON | `i18next` | `.json` |
| JSON5 | `json5` | `.json5` |
| HJSON | `hjson` | `.hjson` |
| YAML (Rails) | `yaml-rails` | `.yml` `.yaml` |
| YAML (Plain) | `yaml-plain` | `.yml` `.yaml` |
| JavaScript | `javascript` | `.js` |
| TypeScript | `typescript` | `.ts` |
| PHP/Laravel | `php-laravel` | `.php` |
| NEON | `neon` | `.neon` |

### Standards & Exchange

| Format | ID | Extensions |
|--------|----|------------|
| XLIFF 1.2 | `xliff` | `.xliff` `.xlf` |
| XLIFF 2.0 | `xliff2` | `.xliff` `.xlf` |
| Gettext PO | `po` | `.po` `.pot` |
| TMX | `tmx` | `.tmx` |
| .NET RESX | `resx` | `.resx` |
| Java Properties | `java-properties` | `.properties` |

### Data & Other

| Format | ID | Extensions |
|--------|----|------------|
| CSV | `csv` | `.csv` `.tsv` |
| Excel | `excel` | `.xlsx` `.xls` |
| TOML | `toml` | `.toml` |
| INI | `ini` | `.ini` |
| SRT Subtitles | `srt` | `.srt` |
| Markdown | `markdown` | `.md` |
| Plain Text | `plain-text` | `.txt` |

### Vendor-Specific

| Format | ID | Extensions |
|--------|----|------------|
| iSpring Suite XLIFF | `ispring-xliff` | `.xliff` `.xlf` |
| Adobe Captivate XML | `captivate-xml` | `.xml` |

## Architecture

All conversions go through a central **Intermediate Representation (IR)**:

```
Source Format  →  Parser  →  IR (I18nResource)  →  Writer  →  Target Format
```

The IR preserves:
- Translation entries with keys and values
- Plurals (zero/one/two/few/many/other)
- Arrays, select/gender, multi-variable plurals
- Comments with roles (developer, translator, extracted)
- Source strings, translation state, max-width constraints
- Format-specific extensions for lossless round-trips

## Adding a New Format

1. **Add the format ID** to `src/ir/resource.rs` (`FormatId` enum)

2. **Add the format extension** to `src/ir/extensions.rs` (`FormatExtension` enum + struct)

3. **Create the parser/writer** at `src/formats/your_format.rs`:

```rust
use super::*;

pub struct Parser;
pub struct Writer;

impl FormatParser for Parser {
    fn detect(&self, extension: &str, content: &[u8]) -> Confidence {
        // Return Definite/High/Low/None based on extension + content inspection
    }

    fn parse(&self, content: &[u8]) -> Result<I18nResource, ParseError> {
        // Parse bytes into IR
    }

    fn capabilities(&self) -> FormatCapabilities {
        // Declare what your format supports
        FormatCapabilities {
            plurals: false,
            nested_keys: true,
            comments: false,
            // ... (14 capability flags total)
            ..Default::default()
        }
    }
}

impl FormatWriter for Writer {
    fn write(&self, resource: &I18nResource) -> Result<Vec<u8>, WriteError> {
        // Convert IR back to bytes
    }

    fn capabilities(&self) -> FormatCapabilities {
        Parser.capabilities()
    }
}
```

4. **Register it** in `src/formats/mod.rs`:

```rust
pub mod your_format;

// In FormatRegistry::new():
Self::register(
    &mut formats,
    "your-format",
    "Your Format Name",
    &[".ext"],
    your_format::Parser,
    your_format::Writer,
);
```

5. **Add tests** at `tests/roundtrip_your_format.rs` with fixtures in `tests/fixtures/your_format/`

6. **Run the full suite:**

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

## Contributing

- Open issues for bugs or feature requests.
- PRs welcome, especially for adding new formats.
- Run `cargo test` before submitting.

## License

MIT License - see [LICENSE](LICENSE) file.

Built by [i18nagent.ai](https://i18nagent.ai)
