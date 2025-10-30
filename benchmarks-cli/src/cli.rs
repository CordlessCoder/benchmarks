#[derive(clap::Parser, Debug)]
/// A simple system information utility focused on performance.
pub struct Args {
    /// Disable this DataProvider. Can be passed multiple times to disable multiple providers
    #[clap(short, long)]
    pub disable: Vec<String>,
    /// Print the names of all data providers, and exit
    #[clap(long)]
    pub print_identifiers: bool,
    /// Make logging output more verbose. By default, only logs at the ERROR level are printed,
    /// but this can be changed by setting the RUST_LOG variable(e.g. `RUST_LOG=info ./sysinfo`)
    #[clap(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}
