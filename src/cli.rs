use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "i18n-convert",
    version,
    about = "Cross-platform localization file format converter"
)]
pub struct Cli {
    /// Input file or directory
    #[arg(required_unless_present = "list_formats")]
    pub input: Option<String>,

    /// Target format alias
    #[arg(short = 't', long, required_unless_present = "list_formats")]
    pub to: Option<String>,

    /// Output file or directory (default: stdout)
    #[arg(short, long)]
    pub out: Option<String>,

    /// Skip data loss confirmation prompts
    #[arg(long)]
    pub force: bool,

    /// Show warnings without writing
    #[arg(long)]
    pub dry_run: bool,

    /// List all supported formats
    #[arg(long)]
    pub list_formats: bool,

    /// Show detailed conversion info
    #[arg(long)]
    pub verbose: bool,
}
