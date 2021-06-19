use {
    log::info,
    std::{
        ffi::OsStr,
        io,
        process::{self, Child, Stdio},
    },
};

pub(crate) struct Command {
    inner: process::Command,
    name: String,
    args: Vec<String>,
}

impl Command {
    pub(crate) fn new<S: AsRef<OsStr>>(program: S) -> Command {
        let inner = process::Command::new(program.as_ref());
        let name: &OsStr = program.as_ref();
        let name = name.to_str().unwrap().to_string();
        let args: Vec<String> = vec![];
        Self { inner, name, args }
    }

    pub(crate) fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Command {
        self.inner.arg(arg.as_ref());
        let arg: &OsStr = arg.as_ref();
        let arg = arg.to_str().unwrap().to_string();
        self.args.push(arg);
        self
    }

    pub(crate) fn args<I, S>(&mut self, args: I) -> &mut Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for arg in args {
            self.arg(arg);
        }
        self
    }

    pub(crate) fn stdin<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Command {
        self.inner.stdin(cfg);
        self
    }

    pub(crate) fn stdout<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Command {
        self.inner.stdout(cfg);
        self
    }

    pub(crate) fn spawn(&mut self) -> io::Result<Child> {
        info!("{} {}", self.name, self.args.join(" "));
        self.inner.spawn()
    }
}
