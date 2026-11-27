use std::{
    fmt,
    fs::File,
    io::{self, Read},
    path::Path,
};

use cap_std::{ambient_authority, fs::Dir};

#[derive(Debug)]
pub(crate) enum BoundedReadError {
    Io(io::Error),
    TooLarge { limit: usize, observed: usize },
}

impl From<io::Error> for BoundedReadError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<BoundedReadError> for io::Error {
    fn from(error: BoundedReadError) -> Self {
        match error {
            BoundedReadError::Io(error) => error,
            too_large @ BoundedReadError::TooLarge { .. } => {
                Self::new(io::ErrorKind::InvalidData, too_large.to_string())
            }
        }
    }
}

impl fmt::Display for BoundedReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => error.fmt(formatter),
            Self::TooLarge { limit, observed } => {
                write!(
                    formatter,
                    "file exceeds {limit}-byte limit ({observed} bytes)"
                )
            }
        }
    }
}

pub(crate) fn read_path(path: &Path, limit: usize) -> Result<Vec<u8>, BoundedReadError> {
    read_file(File::open(path)?, limit)
}

pub(crate) fn read_relative(
    root: &Path,
    relative: &Path,
    limit: usize,
) -> Result<Vec<u8>, BoundedReadError> {
    let directory = Dir::open_ambient_dir(root, ambient_authority())?;
    read_in(&directory, relative, limit)
}

pub(crate) fn read_in(
    directory: &Dir,
    relative: &Path,
    limit: usize,
) -> Result<Vec<u8>, BoundedReadError> {
    read_file(directory.open(relative)?.into_std(), limit)
}

fn read_file(input: File, limit: usize) -> Result<Vec<u8>, BoundedReadError> {
    let metadata = input.metadata()?;
    if !metadata.is_file() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected a regular file").into());
    }
    let observed = usize::try_from(metadata.len()).unwrap_or(usize::MAX);
    if observed > limit {
        return Err(BoundedReadError::TooLarge { limit, observed });
    }
    let mut bytes = Vec::with_capacity(observed);
    input
        .take((limit as u64).saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(BoundedReadError::TooLarge {
            limit,
            observed: bytes.len(),
        });
    }
    Ok(bytes)
}
