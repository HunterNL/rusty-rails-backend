// use core::fmt;
use std::{
    fmt::Debug,
    fs::{self},
    future::Future,
    path::{Path, PathBuf}, // time::Duration,
};

#[derive(Debug)]
pub enum Action {
    Sourced,
    Updated {
        archived: PathBuf,
        new: PathBuf,
        versions: Option<(String, String)>,
    },
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

pub enum UpdateReport {
    NotRequired,
    Required(Option<(String, String)>),
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

/// Errors that can occur when updating the cache, either a file error or something in the user provided function
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error handling file io: {0}")]
    IO(#[from] std::io::Error),

    #[error("Error while calling data source")]
    Source(#[source] Box<dyn std::error::Error>),

    #[error("Error in update deciding function")]
    UpdateFunction(#[source] Box<dyn std::error::Error>),
}

fn append_to_file_stem(path: &mut PathBuf, suffix: &str) -> Result<(), std::io::Error> {
    let mut filename = path
        .file_stem()
        .ok_or(std::io::Error::other("failed to get file stem"))?
        .to_owned();
    let orgin_extension = path.extension();

    filename.push(suffix);
    if let Some(extension) = orgin_extension {
        filename.push(".");
        filename.push(extension);
    }

    path.set_file_name(filename);

    Ok(())
}

impl Cache {
    pub fn new(base_dir: impl AsRef<Path>) -> Result<Self, anyhow::Error> {
        fs::create_dir_all(base_dir.as_ref())?;

        Ok(Self {
            base_dir: base_dir.as_ref().to_owned(),
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

    pub async fn ensure_versioned_async<SourceFn, SourceErr, VersionError, UpdateFn>(
        &self,
        source: SourceFn,
        output_path: impl AsRef<Path>,
        update_fn: UpdateFn,
    ) -> Result<Action, Error>
    where
        SourceFn: SourceAsync<SourceErr>,
        UpdateFn: Fn(&[u8], &[u8]) -> Result<UpdateReport, VersionError>, // E: Error + Send + Sync + 'static,
        SourceErr: Into<Box<dyn std::error::Error>>,
        VersionError: Into<Box<dyn std::error::Error>>,
    {
        let file_path = self.base_dir.join(output_path);
        let source_result = source.get_async().await;
        let remote_content = source_result.map_err(|e: SourceErr| Error::Source(e.into()))?;

        if file_path.try_exists().is_err() || file_path.try_exists().is_ok_and(|f| !f) {
            println!("File path {} seems empty, creating", file_path.display());
            fs::create_dir_all(file_path.parent().expect("valid parent dir"))?;
            fs::write(file_path, remote_content)?;

            Ok(Action::Sourced)
        } else {
            let existing_content = fs::read(&file_path)?;
            let update_required = update_fn(&existing_content, &remote_content)
                .map_err(|e| Error::UpdateFunction(e.into()))?;

            match update_required {
                UpdateReport::NotRequired => Ok(Action::Skipped),
                UpdateReport::Required(version_info) => {
                    let archived_path = Self::archive_file(&file_path)?;
                    fs::write(&file_path, &remote_content)?;

                    Ok(Action::Updated {
                        archived: archived_path,
                        new: file_path,
                        versions: version_info,
                    })
                }
            }
        }
    }

    pub async fn ensure_async<SourceFn, SourceErr>(
        &self,
        source: SourceFn,
        output_path: impl AsRef<Path>,
    ) -> Result<Action, Error>
    where
        SourceFn: SourceAsync<SourceErr>,
        SourceErr: Into<Box<dyn std::error::Error>>,
    {
        let file_path = self.base_dir.join(output_path);

        if !file_path.exists() {
            let remote_content = source
                .get_async()
                .await
                .map_err(|a| Error::Source(a.into()))?;

            fs::write(file_path, remote_content)?;

            Ok(Action::Sourced)
        } else {
            Ok(Action::Skipped)
        }
    }
}

#[cfg(test)]
mod tests {

    use super::Error as SuperError;
    use std::error::Error;

    #[test]
    fn source_err() {
        // let c = Cache::new("./cache").unwrap();
        // c.ensure_async(source, output_path)

        let a = SuperError::UpdateFunction("foo".into());
        let source = a.source().expect("source to be set").to_string();
        assert_eq!(source, "foo".to_owned())
    }
}
