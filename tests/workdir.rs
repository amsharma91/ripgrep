use std::env;
use std::error;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::str::FromStr;
use std::sync::atomic::{ATOMIC_USIZE_INIT, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

static TEST_DIR: &'static str = "ripgrep-tests";
static NEXT_ID: AtomicUsize = ATOMIC_USIZE_INIT;

/// WorkDir represents a directory in which tests are run.
///
/// Directories are created from a global atomic counter to avoid duplicates.
#[derive(Debug)]
pub struct WorkDir {
    /// The directory in which this test executable is running.
    root: PathBuf,
    /// The directory in which the test should run. If a test needs to create
    /// files, they should go in here.
    dir: PathBuf,
}

impl WorkDir {
    /// Create a new test working directory with the given name. The name
    /// does not need to be distinct for each invocation, but should correspond
    /// to a logical grouping of tests.
    pub fn new(name: &str) -> WorkDir {
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let root = env::current_exe().unwrap()
            .parent().expect("executable's directory").to_path_buf();
        let dir = root.join(TEST_DIR).join(name).join(&format!("{}", id));
        nice_err(&dir, repeat(|| fs::create_dir_all(&dir)));
        WorkDir {
            root: root,
            dir: dir,
        }
    }

    /// Create a new file with the given name and contents in this directory.
    pub fn create<P: AsRef<Path>>(&self, name: P, contents: &str) {
        let path = self.dir.join(name);
        let mut file = nice_err(&path, File::create(&path));
        nice_err(&path, file.write_all(contents.as_bytes()));
        nice_err(&path, file.flush());
    }

    /// Remove a file with the given name from this directory.
    pub fn remove<P: AsRef<Path>>(&self, name: P) {
        let path = self.dir.join(name);
        nice_err(&path, fs::remove_file(&path));
    }

    /// Create a new directory with the given path (and any directories above
    /// it) inside this directory.
    pub fn create_dir<P: AsRef<Path>>(&self, path: P) {
        let path = self.dir.join(path);
        nice_err(&path, repeat(|| fs::create_dir_all(&path)));
    }

    /// Creates a new command that is set to use the ripgrep executable in
    /// this working directory.
    pub fn command(&self) -> process::Command {
        let mut cmd = process::Command::new(&self.bin());
        cmd.current_dir(&self.dir);
        cmd
    }

    /// Returns the path to the ripgrep executable.
    pub fn bin(&self) -> PathBuf {
        self.root.join("rg")
    }

    /// Returns the path to this directory.
    pub fn path(&self) -> &Path {
        &self.dir
    }

    /// Creates a directory symlink to the src with the given target name
    /// in this directory.
    #[cfg(not(windows))]
    pub fn link<S: AsRef<Path>, T: AsRef<Path>>(&self, src: S, target: T) {
        use std::os::unix::fs::symlink;
        let src = self.dir.join(src);
        let target = self.dir.join(target);
        let _ = fs::remove_file(&target);
        nice_err(&target, symlink(&src, &target));
    }

    #[cfg(windows)]
    pub fn link<S: AsRef<Path>, T: AsRef<Path>>(&self, src: S, target: T) {
        use std::os::windows::fs::symlink_dir;
        let src = self.dir.join(src);
        let target = self.dir.join(target);
        let _ = fs::remove_dir(&target);
        nice_err(&target, symlink_dir(&src, &target));
    }

    /// Runs and captures the stdout of the given command.
    ///
    /// If the return type could not be created from a string, then this
    /// panics.
    pub fn stdout<E: fmt::Debug, T: FromStr<Err=E>>(
        &self,
        cmd: &mut process::Command,
    ) -> T {
        let o = self.output(cmd);
        let stdout = String::from_utf8_lossy(&o.stdout);
        match stdout.parse() {
            Ok(t) => t,
            Err(err) => {
                panic!("could not convert from string: {:?}\n\n{}", err, stdout);
            }
        }
    }

    /// Gets the output of a command. If the command failed, then this panics.
    pub fn output(&self, cmd: &mut process::Command) -> process::Output {
        let o = cmd.output().unwrap();
        if !o.status.success() {
            let suggest =
                if o.stderr.is_empty() {
                    "\n\nDid your search end up with no results?".to_string()
                } else {
                    "".to_string()
                };

            panic!("\n\n==========\n\
                    command failed but expected success!\
                    {}\
                    \n\ncommand: {:?}\
                    \ncwd: {}\
                    \n\nstatus: {}\
                    \n\nstdout: {}\
                    \n\nstderr: {}\
                    \n\n==========\n",
                   suggest, cmd, self.dir.display(), o.status,
                   String::from_utf8_lossy(&o.stdout),
                   String::from_utf8_lossy(&o.stderr));
        }
        o
    }

    /// Runs the given command and asserts that it resulted in an error exit
    /// code.
    pub fn assert_err(&self, cmd: &mut process::Command) {
        let o = cmd.output().unwrap();
        if o.status.success() {
            panic!("\n\n===== {:?} =====\n\
                    command succeeded but expected failure!\
                    \n\ncwd: {}\
                    \n\nstatus: {}\
                    \n\nstdout: {}\n\nstderr: {}\
                    \n\n=====\n",
                   cmd, self.dir.display(), o.status,
                   String::from_utf8_lossy(&o.stdout),
                   String::from_utf8_lossy(&o.stderr));
        }
    }
}

fn nice_err<P: AsRef<Path>, T, E: error::Error>(
    path: P,
    res: Result<T, E>,
) -> T {
    match res {
        Ok(t) => t,
        Err(err) => {
            panic!("{}: {:?}", path.as_ref().display(), err);
        }
    }
}

fn repeat<F: FnMut() -> io::Result<()>>(mut f: F) -> io::Result<()> {
    let mut last_err = None;
    for _ in 0..10 {
        if let Err(err) = f() {
            last_err = Some(err);
            thread::sleep(Duration::from_millis(500));
        } else {
            return Ok(());
        }
    }
    Err(last_err.unwrap())
}
