use std::fs;
use std::io::Write;
use std::path::Path;

use crate::Error;

pub(crate) fn write_secret(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)?.write_all(bytes)?;
    Ok(())
}

pub(crate) fn read_secret(path: &Path) -> Result<Vec<u8>, Error> {
    Ok(fs::read(path)?)
}
