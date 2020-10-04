use {
    crate::{
        config::Config,
        errors::{ALIError, ALIResult, ErrorKind},
        utils::check_su,
    },
    std::{
        env::var,
        fs::{self, create_dir_all, read_dir, File},
        io::prelude::*,
        os::unix::fs::PermissionsExt,
        path::PathBuf,
        process::Command,
    },
};

enum ProfileVariant {
    Releng,
    Baseline,
}

struct Profile<'a> {
    _variant: ProfileVariant, // TODO: remove if useless
    name: &'a str,
}

impl<'a> Profile<'a> {
    fn from_str(name: &'a str) -> Option<Profile> {
        let _variant = match name {
            "releng" => ProfileVariant::Releng,
            "baseline" => ProfileVariant::Baseline,
            _ => return None,
        };
        Some(Profile { _variant, name })
    }

    fn path(&self) -> PathBuf {
        let mut path = PathBuf::from("/usr/share/archiso/configs");
        path.push(self.name);
        path
    }

    fn name(&self) -> &str {
        self.name
    }
}

struct LiveCreator<'a> {
    profile: Profile<'a>,
    working_dir: PathBuf,
    profile_root: PathBuf,
    installer_target: PathBuf,
    iso_location: Option<PathBuf>,
}

impl<'a> LiveCreator<'a> {
    fn new(profile: Profile<'a>, config: &'a Config) -> LiveCreator<'a> {
        let working_dir = config.live_cd().working_dir().clone();
        let mut installer_target = working_dir.clone();
        installer_target.push(&profile.name());
        installer_target.push("airootfs/root/installer");

        let mut profile_root = working_dir.clone();
        profile_root.push(&profile.name());

        LiveCreator {
            profile,
            working_dir,
            installer_target,
            profile_root,
            iso_location: None,
        }
    }

    fn create_working_dir(&mut self) -> &mut Self {
        create_dir_all(&self.working_dir).unwrap();
        self
    }

    fn check_umask(&mut self) -> ALIResult<&mut Self> {
        let file = File::open(&self.working_dir).unwrap();
        let metadata = file.metadata().unwrap();
        let mode = metadata.permissions().mode();
        let required_mode = 0o40755;

        if mode == required_mode {
            Ok(self)
        } else {
            Err(ALIError::new(ErrorKind::InvalidUmask {
                expected: required_mode,
                got: mode,
            }))
        }
    }

    fn copy_archiso_files(&mut self) -> &mut Self {
        let profile_path = self.profile.path();
        let profile_path = profile_path.to_str().unwrap();
        let working_dir = self.working_dir.to_str().unwrap();

        Command::new("cp")
            .args(&["-rv", profile_path, working_dir])
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
        self
    }

    fn copy_installer(&mut self, source: PathBuf) -> &mut Self {
        Command::new("cp")
            .arg("-rv")
            .arg(source)
            .arg(&self.installer_target)
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
        self
    }

    fn add_packages(&mut self, packages: Vec<&str>) -> &mut Self {
        let mut target_location = self.profile_root.clone();
        target_location.push("packages.x86_64");

        let mut target = fs::OpenOptions::new()
            .append(true)
            .open(&target_location)
            .unwrap();

        packages.into_iter().for_each(|package| {
            target.write(package.as_bytes()).unwrap();
            target.write(b"\n").unwrap();
        });
        self
    }

    fn build(&mut self) -> &mut Self {
        let mut program = self.profile_root.clone();
        program.push("build.sh");

        let mut work_dir = self.profile_root.clone();
        work_dir.push("work");
        let work_dir = work_dir.to_str().unwrap();

        let mut out_dir = self.profile_root.clone();
        out_dir.push("out");
        let out_dir = out_dir.to_str().unwrap();

        Command::new(&program)
            .args(&["-v", "-w", work_dir, "-o", out_dir])
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
        self
    }

    fn copy_iso(&mut self) -> &mut Self {
        let mut iso_dir = self.profile_root.clone();
        iso_dir.push("out");
        let iso = read_dir(&iso_dir).unwrap().next().unwrap().unwrap();
        let iso_name = iso.file_name();
        let iso_source = iso.path();
        let mut iso_target = self.working_dir.clone();
        iso_target.push(&iso_name);
        fs::copy(&iso_source, &iso_target).unwrap();
        self.iso_location = Some(iso_target);
        self
    }

    fn change_iso_owner(&mut self) -> &mut Self {
        let user = match var("SUDO_USER") {
            Ok(user) => user,
            Err(_) => return self,
        };
        let iso_location = self.iso_location.as_ref().unwrap();
        let iso_location = iso_location.to_str().unwrap();
        let permissions = format!("{}:{}", user, user);

        Command::new("chown")
            .args(&[&permissions, iso_location])
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
        self
    }

    fn finish(&mut self) {
        let iso_location = self.iso_location.as_ref().unwrap();
        let iso_location = iso_location.to_str().unwrap();
        println!("Done! {}", iso_location);
    }
}

pub fn main(config: &Config) -> ALIResult<()> {
    check_su()?;
    let profile_name = config.live_cd().profile();

    let profile =
        Profile::from_str(&profile_name).ok_or(ALIError::new(ErrorKind::UnknownArchISOProfile))?;

    LiveCreator::new(profile, config)
        .create_working_dir()
        .check_umask()?
        .copy_archiso_files()
        .copy_installer(config.live_cd().installer_location())
        .add_packages(config.packages().archiso())
        .build()
        .copy_iso()
        .change_iso_owner()
        .finish();
    Ok(())
}
