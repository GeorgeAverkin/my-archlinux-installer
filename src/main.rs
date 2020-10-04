use {
    clap::{App, AppSettings, Arg, ArgMatches},
    rust_archlinux_installer::{
        chroot_install, config::Config, errors::ALIResult, install, live_cd, utils::exe_dir,
    },
    std::{fs::File, io::prelude::*, path::PathBuf, process::exit},
};

mod subcommands {
    use clap::{App, Arg, SubCommand};

    pub fn config() -> App<'static, 'static> {
        SubCommand::with_name("mkconfig").about("Emits config template")
    }

    pub fn live_cd() -> App<'static, 'static> {
        SubCommand::with_name("archiso")
            .about("Builds Live-CD")
            .arg(
                Arg::with_name("working dir")
                    .long("working-dir")
                    .value_name("PATH")
                    .help("Specifies location of build directory"),
            )
    }

    pub fn install() -> App<'static, 'static> {
        SubCommand::with_name("install").about("Begins install")
    }

    pub fn chroot_install() -> App<'static, 'static> {
        SubCommand::with_name("chroot-install").about("Begins install in chroot environment")
    }
}

fn get_subcommands() -> Vec<App<'static, 'static>> {
    use subcommands::*;
    vec![config(), live_cd(), install(), chroot_install()]
}

fn run(matches: &ArgMatches, conf_path: &mut PathBuf) -> ALIResult<()> {
    let mut config = || Config::new(conf_path, matches);

    if matches.is_present("mkconfig") {
        let mut file = File::create("config.toml").unwrap();

        file.write_all(include_bytes!("../res/config.toml"))
            .unwrap();
    }
    if matches.is_present("archiso") {
        live_cd::main(&config()?)?;
    }
    if matches.is_present("install") {
        install::main(&config()?)?;
    }
    if matches.is_present("chroot-install") {
        chroot_install::main(&config()?)?;
    }
    Ok(())
}

fn main() {
    let app = App::new("Arch Linux installer")
        .about("Arch Linux installation program, written in Rust.")
        .version("0.0.1")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommands(get_subcommands())
        .arg(
            Arg::with_name("config")
                .long("config")
                .short("c")
                .value_name("FILE")
                .help("Specifies location of config file")
                .takes_value(true),
        );
    let matches = app.get_matches();

    let mut config_path = matches
        .value_of("config")
        .map(|location| PathBuf::from(location))
        .unwrap_or({
            let mut exe = exe_dir();
            exe.push("config.toml");
            exe
        });

    if let Err(e) = run(&matches, &mut config_path) {
        eprintln!("{}", e);
        exit(1);
    };
}
