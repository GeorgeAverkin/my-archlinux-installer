use {
    clap,
    rust_archlinux_installer::{
        chroot_install,
        config::{Config, InstallStepRange},
        errors::ALIResult,
        install, live_cd,
        log::init_logger,
        utils::exe_dir,
    },
    std::{fs::File, io::prelude::*, path::PathBuf, process::exit},
};

use clap::StructOpt;
use rust_archlinux_installer::config::InstallStep;

const STEPS_HELP: &str = "Specifies installation step to run
Values:
    partition
    encrypt
    format
    mount
    mirrors
    multilib
    pacstrap
    fstab
    chroot";

#[derive(clap::Parser, Debug)]
enum Command {
    #[clap(name = "mkconfig", about = "Emits config template")]
    MkConfig,

    #[clap(name = "archiso", about = "Builds Live-CD")]
    ArchISO {
        #[clap(
            long,
            value_name = "PATH",
            help = "Specifies location of build directory"
        )]
        working_dir: PathBuf,
    },

    #[clap(about = "Begins install")]
    Install {
        #[clap(long, default_value = "..", help = STEPS_HELP, value_name = "[step] | [from]..[to]")]
        steps: InstallStepRange,
    },

    #[clap(about = "Begins install in chroot environment")]
    ChrootInstall {},
}

#[derive(clap::Parser, Debug)]
#[clap(
    name = "Arch Linux installer",
    about = "Arch Linux installation program, written in Rust.",
    version = "0.1.0"
)]
struct Args {
    #[clap(subcommand)]
    command: Command,

    #[clap(
        short,
        long,
        value_name = "FILE",
        help = "Specifies location of config file"
    )]
    config: Option<PathBuf>,
}

fn run(args: &Args, conf_path: &mut PathBuf) -> ALIResult<()> {
    init_logger();

    match args.command {
        // TODO: check steps
        Command::MkConfig => {
            let mut file = File::create("config.toml").unwrap();

            file.write_all(include_bytes!("../res/config.toml"))
                .unwrap();
        }
        Command::ArchISO { ref working_dir } => {
            let config = Config::new(conf_path, Some(working_dir.clone()))?;
            live_cd::main(&config)?;
        }
        Command::Install { ref steps } => {
            println!("Partition: {}", steps.contains(&InstallStep::Partition));
            println!("Encrypt: {}", steps.contains(&InstallStep::Encrypt));
            println!("Format: {}", steps.contains(&InstallStep::Format));
            println!("Mount: {}", steps.contains(&InstallStep::Mount));
            println!("Mirrors: {}", steps.contains(&InstallStep::Mirrors));
            println!("Multilib: {}", steps.contains(&InstallStep::Multilib));
            println!("Pacstrap: {}", steps.contains(&InstallStep::Pacstrap));
            println!("Fstab: {}", steps.contains(&InstallStep::Fstab));
            println!("Chroot: {}", steps.contains(&InstallStep::Chroot));

            let mut config = Config::new(conf_path, None)?;
            config.validate()?;
            install::main(&mut config, steps)?;
        }
        Command::ChrootInstall {} => {
            let config = Config::new(conf_path, None)?;
            config.validate()?;
            chroot_install::main(&config)?;
        }
    }
    Ok(())
}

fn main() {
    let args = Args::parse();

    let mut config_path = match args.config {
        Some(ref config) => config.clone(),
        None => {
            let mut exe = exe_dir();
            exe.push("config.toml");
            exe
        }
    };
    if let Err(e) = run(&args, &mut config_path) {
        eprintln!("{}", e);
        exit(1);
    };
}
