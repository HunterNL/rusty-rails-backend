// use core::fmt;
use std::{
    error::Error,
    fmt::{Debug, Display},
    fs::{self},
    future::Future,
    path::{self, Path, PathBuf}, // time::Duration,
};

// use derive_more::derive::From;
use thiserror::Error;

#[derive(Debug)]
pub enum Action {
    Sourced,
    Updated { archived: PathBuf, new: PathBuf },
    Skipped,
}

pub struct Cache {
    pub base_dir: PathBuf,
}

pub trait SourceAsync<E: Into<Box<dyn std::error::Error>>> {
    async fn get_async(&self) -> Result<Vec<u8>, E>;
}

pub trait Source<E> {
    fn get(&self) -> Result<Vec<u8>, E>;
}

/// Implementations on bare functions for convinience
impl<E, F, Fut> SourceAsync<E> for F
where
    E: Into<Box<dyn std::error::Error>>,
    F: Fn() -> Fut,
    Fut: Future<Output = Result<Vec<u8>, E>>,
{
    async fn get_async(&self) -> Result<Vec<u8>, E> {
        self().await
    }
}

/// Implementations on bare functions for convinience
impl<E, F> Source<E> for F
where
    F: Fn() -> Result<Vec<u8>, E>,
{
    fn get(&self) -> Result<Vec<u8>, E> {
        self()
    }
}

/// An error encountered while retrieving the data to cache, as the function is user provided the error can be anything
#[derive(Debug)]
pub struct SourceError {
    // source: Box<dyn std::error::Error>,
}

impl Error for SourceError {
    // fn source(&self) -> Option<&(dyn Error + 'static)> {
    //     Some(self.source()).as_deref()
    // }
}

impl Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // {
        //     let this = &self.source;
        //     fmt::Debug::fmt(&**this, f)
        // }
        f.write_str("sourceerror")
    }
}

impl From<Box<dyn std::error::Error>> for SourceError {
    fn from(err: Box<(dyn std::error::Error + 'static)>) -> Self {
        SourceError {}
    }
}

/// Errors that can occur when updating the cache, either a file error or something in the user provided function
#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Error handling file io: {0}")]
    IOError(#[from] std::io::Error),

    #[error("Error in source")]
    SourceError,

    #[error("Error in update fn")]
    UpdateFunctionError,
    // #[error("Error in source: {0}")]
}

fn append_to_file_stem(path: &mut PathBuf, suffix: &str) -> Result<(), std::io::Error> {
    let mut filename = path
        .file_stem()
        .ok_or(std::io::Error::other("failed to get file stem"))?
        .to_owned();
    let orgin_extension = path.extension();

    filename.push(suffix);
    if orgin_extension.is_some() {
        filename.push(".");
        filename.push(&orgin_extension.unwrap());
    }

    path.set_file_name(filename);

    Ok(())
}

impl Cache {
    pub fn new(base_dir: &Path) -> Result<Self, anyhow::Error> {
        fs::create_dir_all(base_dir)?;

        Ok(Self {
            base_dir: base_dir.into(),
        })
    }

    fn archive_file(filepath: impl AsRef<Path>) -> Result<PathBuf, std::io::Error> {
        const MAX_FREE_PATH_ATTEMPTS: u16 = 512;

        for n in 1..MAX_FREE_PATH_ATTEMPTS {
            let mut path_candidate = filepath.as_ref().to_owned();
            append_to_file_stem(
                &mut path_candidate,
                ("_".to_owned() + &n.to_string()).as_str(),
            )?;

            println!("Path candidate: {}", path_candidate.display());
            let is_empty = path_candidate.try_exists().is_ok_and(|exists| !exists);
            println!("is empty: {:?}", is_empty);
            if is_empty {
                println!(
                    "Renaming from {} to {}",
                    filepath.as_ref().display(),
                    path_candidate.display()
                );
                std::fs::rename(filepath, &path_candidate)?;
                return Ok(path_candidate);
            }
        }

        Err(std::io::Error::other("out of possible candidate paths"))
    }

    // #[allow(dead_code)]
    // pub fn ensure<S, E>(&self, source: S, output_path: &Path) -> Result<Action, CacheError<E>>
    // where
    //     S: Source<E>,
    //     // E: Error + Send + Sync + 'static,
    // {
    //     let file_path = self.base_dir.join(output_path);
    //     let content = source.get().map_err(CacheError::SourceError)?;
    //     let existing_content = fs::read(file_path).map_err(op)

    //     if !file_path.exists() {
    //         // No file exists, just save the data as we got it
    //         let mut file = File::create(&file_path).map_err(CacheError::IOError)?;
    //         fs::write(file_path, content).map_err(CacheError::IOError)?;
    //         // file.write_all(&content).
    //         Ok(Action::Fetched)
    //     } else {
    //         // File exists

    //     }

    //     Ok(Action::Updated)
    // }

    pub async fn ensure_versioned_async<SourceFn, SourceErr, VersionError, UpdateFn>(
        &self,
        source: SourceFn,
        output_path: impl AsRef<Path>,
        update_fn: UpdateFn,
    ) -> Result<Action, CacheError>
    where
        SourceFn: SourceAsync<SourceErr>,
        UpdateFn: Fn(&[u8], &[u8]) -> Result<bool, VersionError>, // E: Error + Send + Sync + 'static,
        SourceErr: Into<Box<dyn std::error::Error>>,
        VersionError: Into<Box<dyn std::error::Error>>,
    {
        let file_path = self.base_dir.join(output_path);
        let source_result = source.get_async().await;
        let remote_content = source_result.map_err(|e: SourceErr| CacheError::SourceError)?;

        println!("downloaded");

        if !file_path.exists() {
            println!("no exsists");
            fs::write(file_path, remote_content)?;

            Ok(Action::Sourced)
        } else {
            println!("no exists");
            let existing_content = fs::read(&file_path)?;
            // File exists
            let update_required = update_fn(&existing_content, &remote_content)
                .map_err(|e| CacheError::UpdateFunctionError)?;

            if update_required {
                println!("pre archive {}", &file_path.display());
                let archived_path = Self::archive_file(&file_path)?;
                println!("post archive");
                fs::write(&file_path, &remote_content)?;

                Ok(Action::Updated {
                    archived: archived_path,
                    new: file_path,
                })
            } else {
                println!("no update required");
                Ok(Action::Skipped)
            }
        }
    }

    pub async fn ensure_async<SourceFn, SourceErr>(
        &self,
        source: SourceFn,
        output_path: impl AsRef<Path>,
    ) -> Result<Action, CacheError>
    where
        SourceFn: SourceAsync<SourceErr>,
        SourceErr: Into<Box<dyn Error>>,
    {
        let file_path = self.base_dir.join(output_path);

        if !file_path.exists() {
            let remote_content = source
                .get_async()
                .await
                .map_err(|a| CacheError::SourceError)?;

            fs::write(file_path, remote_content)?;

            Ok(Action::Sourced)
        } else {
            Ok(Action::Skipped)
        }
    }
}
