use crate::errors::{CommandExecutionError, SudoRequiredError};

use {
    crate::errors::ALIResult,
    std::{
        env::{current_exe, var},
        fmt,
        fs::File,
        io::{prelude::*, stdin},
        path::{Path, PathBuf},
    },
};

pub mod command;

use command::Command;

pub fn sudo_passwd_off(user: &str) {
    let mut buffer = String::new();

    File::open("/etc/sudoers")
        .unwrap()
        .read_to_string(&mut buffer)
        .unwrap();

    buffer = buffer.replace(
        &format!("\n{} ALL=(ALL) ALL", user),
        &format!("\n{} ALL=(ALL) NOPASSWD: ALL", user),
    );
    File::create("/etc/sudoers")
        .unwrap()
        .write_all(buffer.as_bytes())
        .unwrap();
}

pub fn sudo_passwd_on(user: &str) {
    let mut buffer = String::new();

    File::open("/etc/sudoers")
        .unwrap()
        .read_to_string(&mut buffer)
        .unwrap();

    buffer = buffer.replace(
        &format!("\n{} ALL=(ALL) NOPASSWD: ALL", user),
        &format!("\n{} ALL=(ALL) ALL", user),
    );
    File::create("/etc/sudoers")
        .unwrap()
        .write_all(buffer.as_bytes())
        .unwrap();
}

pub fn su_command(user: &str, command: &str, args: &[&str]) -> Command {
    let mut eval_cmd = vec![command];
    args.into_iter().for_each(|arg| eval_cmd.push(arg));
    let eval_cmd = eval_cmd.join(" ");
    let mut cmd = Command::new("su");
    cmd.args(&["-c", &eval_cmd, user]);
    cmd
}

pub fn pacman_install(packages: &[&str]) {
    let status = Command::new("pacman")
        .args(&["-S", "--noconfirm", "--needed"])
        .args(packages)
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    assert!(status.success());
}

pub fn check_su() -> ALIResult<()> {
    let user = var("USER");

    if let Ok(user) = user {
        if user == "root" {
            return Ok(());
        }
    }
    Err(SudoRequiredError {}.into())
}

pub fn exe_dir() -> PathBuf {
    let mut path = current_exe().unwrap();
    path.pop();
    path
}

pub fn answer<P: AsRef<str>>(question: P) -> bool {
    println!("{} [y/n]", question.as_ref());

    loop {
        let mut buffer = String::new();
        stdin().read_line(&mut buffer).unwrap();
        buffer = buffer.to_lowercase().trim().to_owned();

        if buffer == "y" {
            return true;
        }
        if buffer == "n" {
            return false;
        }
        println!("Please, answer [y/n].");
    }
}

pub fn git_clone<P: AsRef<Path>>(repo: &str, dir: P) -> ALIResult<()> {
    let status = Command::new("git")
        .args(&["clone", repo])
        .arg(dir.as_ref())
        .spawn()?
        .wait()?;

    if status.success() {
        return Err(CommandExecutionError(format!("git clone {} {:?}", repo, dir.as_ref())).into());
    }
    Ok(())
}

pub struct Service<'a>(pub &'a str);

impl<'a> Service<'a> {
    pub fn enable(self) {
        let status = Command::new("systemctl")
            .args(&["enable", self.0])
            .spawn()
            .unwrap()
            .wait()
            .unwrap();

        assert!(status.success());
    }
}

#[derive(Debug)]
pub struct FileSystem {
    spec: String,
    mountpoint: PathBuf,
}

impl fmt::Display for FileSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let buffer = format!("{} {}", self.spec, self.mountpoint.to_str().unwrap());
        write!(f, "{}", buffer)
    }
}

#[derive(Debug)]
pub struct Mounted {
    file_systems: Vec<FileSystem>,
}

impl Mounted {
    pub fn new() -> Mounted {
        let mut src_file = File::open("/proc/mounts").unwrap();
        let mut buffer = String::new();
        src_file.read_to_string(&mut buffer).unwrap();
        let mut file_systems: Vec<FileSystem> = Vec::new();

        for line in buffer.lines() {
            let mut columns = line.split_whitespace();
            let spec = columns.next().unwrap().to_owned();
            let mountpoint = columns.next().unwrap().to_owned();
            let mountpoint = PathBuf::from(mountpoint);
            file_systems.push(FileSystem { spec, mountpoint });
        }

        Mounted { file_systems }
    }

    pub fn find_by_mountpoint<P: AsRef<Path>>(&self, mountpoint: P) -> Option<&FileSystem> {
        let mountpoint = mountpoint.as_ref();

        let fs = self.file_systems.iter().find(|fs| {
            let item: &Path = fs.mountpoint.as_ref();
            mountpoint == item
        });
        fs
    }
}

impl fmt::Display for Mounted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buffer = String::new();

        for partition in &self.file_systems {
            buffer = format!("{}\n{}", buffer, partition);
        }
        write!(f, "{}", buffer)
    }
}
