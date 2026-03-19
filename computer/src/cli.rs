use clap::Parser;

#[derive(Parser)]
#[command(about = "Hack computer simulator")]
pub struct Args {
    /// Assembly source file (.asm)
    pub path: String,

    /// Print trace of function calls and game frames
    #[arg(long)]
    pub trace: bool,

    /// Print detailed state at every labeled address
    #[arg(long)]
    pub verbose: bool,

    /// Print the circuit graph and wiring
    #[arg(long)]
    pub print: bool,

    /// Load and synthesize the chip but don't execute
    #[arg(long)]
    pub no_exec: bool,

    /// Double the window size
    #[arg(long = "2x")]
    pub scale_2x: bool,
}

impl Args {
    pub fn scale(&self) -> usize {
        if self.scale_2x { 2 } else { 1 }
    }
}
