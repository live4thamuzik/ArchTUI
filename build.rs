use clap::CommandFactory;
use std::fs;
use std::path::PathBuf;

// Re-use the CLI definition from src/cli.rs.
// cli.rs imports PathBuf and clap types which are already imported above.
#[allow(unused_imports, dead_code)]
mod cli {
    include!("src/cli.rs");
}
use cli::Cli;

fn main() {
    // Re-run if the env var or CLI definition changes
    println!("cargo:rerun-if-env-changed=ARCHTUI_GEN_DIR");
    println!("cargo:rerun-if-changed=src/cli.rs");

    // Only generate when explicitly requested via env var (not on every build)
    let out_dir = match std::env::var("ARCHTUI_GEN_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => return,
    };

    // Generate man page
    let man_dir = out_dir.join("man");
    fs::create_dir_all(&man_dir).expect("Failed to create man directory");
    let cmd = Cli::command();
    let man = clap_mangen::Man::new(cmd);
    let mut buf = Vec::new();
    man.render(&mut buf).expect("Failed to render man page");
    fs::write(man_dir.join("archtui.1"), buf).expect("Failed to write man page");

    // Generate shell completions
    let comp_dir = out_dir.join("completions");
    fs::create_dir_all(&comp_dir).expect("Failed to create completions directory");
    let mut cmd = Cli::command();
    for shell in [
        clap_complete::Shell::Bash,
        clap_complete::Shell::Zsh,
        clap_complete::Shell::Fish,
    ] {
        clap_complete::generate_to(shell, &mut cmd, "archtui", &comp_dir)
            .expect("Failed to generate completions");
    }
}
