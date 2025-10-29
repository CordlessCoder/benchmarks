use bstr::{ByteSlice, io::BufReadExt};
pub use rxfetch::components::name::PwuIdErr;
use rxfetch::{
    components::name::{PwuId, current_uid},
    display::DisplayBytes,
};
use std::{
    ffi::OsStr,
    io::BufReader,
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    process::{Command, Stdio},
    time::Duration,
};
use timeout_readwrite::TimeoutReader;

#[derive(Debug, Clone)]
pub struct Shell {
    pub path: PathBuf,
    pub version: Option<String>,
}

// These shells are known to respond to --version
static KNOWN_POSIX_SHELLS: &[&str] = &["sh", "zsh", "ksh", "csh", "tcsh", "bash", "fish"];
const READ_TIMEOUT: Duration = Duration::from_secs(1);

fn extract_program_version_from_command(mut command: Command) -> Option<String> {
    let mut process = command
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .ok()?;
    let output = process.stdout.take().unwrap();
    let with_timeout = TimeoutReader::new(output, READ_TIMEOUT);
    let mut buffered = BufReader::new(with_timeout);
    let mut version = None;
    _ = buffered.for_byte_line(|line| {
        let Some((_, after_version)) = line.split_once_str("version") else {
            return Ok(true);
        };
        let start = after_version.find_byteset("0123456789.").unwrap_or(0);
        let version_candidate = &after_version[start..];
        let len = version_candidate
            .iter()
            .take_while(|b| matches!(b, b'0'..=b'9' | b'.'))
            .count();
        let version_candidate = core::str::from_utf8(&version_candidate[..len]).unwrap();
        if version_candidate.is_empty() {
            return Ok(false);
        }
        version = Some(version_candidate.to_string());
        Ok(false)
    });
    _ = process.kill();
    _ = process.wait();
    version
}

impl Shell {
    pub fn from_path(path: PathBuf) -> Self {
        let name = path.file_name();
        let mut version = None;
        let is_posix = name.is_some_and(|name| {
            KNOWN_POSIX_SHELLS
                .iter()
                .any(|posix| name.as_bytes() == posix.as_bytes())
        });
        if is_posix {
            let mut command = Command::new(path.as_os_str());
            command.arg("--version");
            version = extract_program_version_from_command(command);
        }
        Shell { path, version }
    }
    pub fn name(&self) -> DisplayBytes<'_> {
        let bytes = self
            .path
            .file_name()
            .map(|n| n.as_bytes())
            .unwrap_or(b"UNKNOWN");
        DisplayBytes::new(bytes)
    }
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }
}

#[derive(Debug, Clone)]
pub struct UserData {
    pub username: String,
    pub home: PathBuf,
    pub shell: Shell,
}

impl UserData {
    fn fetch_from_pwuid<B>(id: &PwuId<B>) -> Self {
        Self {
            username: id.name().to_string(),
            home: PathBuf::from(OsStr::from_bytes(&id.dir().0)),
            shell: Shell::from_path(PathBuf::from(OsStr::from_bytes(&id.shell().0))),
        }
    }
    fn try_fetch_noalloc() -> Result<Self, PwuIdErr> {
        let buf = [0_u8; 4096];
        let id = PwuId::try_get(buf, current_uid()).map_err(|(err, _)| err)?;
        Ok(Self::fetch_from_pwuid(&id))
    }
    pub fn fetch() -> Result<Self, PwuIdErr> {
        // Attempt to fetch data without allocating first.
        match Self::try_fetch_noalloc() {
            Err(PwuIdErr::BufferTooSmall) => {
                // We have to allocate
                PwuId::get_alloc(current_uid()).map(|id| Self::fetch_from_pwuid(&id))
            }
            result => result,
        }
    }
}
