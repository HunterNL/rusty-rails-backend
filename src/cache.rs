use std::{
    fmt::Debug,
    fs::{self, File},
    future::Future,
    io::Write,
    path::{Path, PathBuf}, // time::Duration,
};

#[derive(Debug)]
pub enum Action {
    Updated,
    Skipped,
}

pub struct Cache {
    base_dir: PathBuf,
}

#[derive(thiserror::Error, Debug)]
pub enum CacheError<E>
where
// E1: Error,
{
    #[error("File exsists")]
    #[allow(dead_code)]
    FileExists,
    #[error("Error handling file io: {0}")]
    IOError(std::io::Error),
    #[error("Error from data source: {0}")]
    SourceError(E),
}

impl Cache {
    pub fn new(base_dir: PathBuf) -> Result<Self, anyhow::Error> {
        fs::create_dir_all(&base_dir)?;

        Ok(Self { base_dir })
    }

    #[allow(dead_code)]
    pub fn ensure<E>(
        &self,
        source: impl Fn() -> Result<Vec<u8>, E>,
        output_path: &Path,
    ) -> Result<Action, CacheError<E>> {
        let file_path = self.base_dir.join(output_path);

        if file_path.exists() {
            return Ok(Action::Skipped);
        }

        let content = source().map_err(CacheError::SourceError)?;

        let mut file = File::create(&file_path).map_err(CacheError::IOError)?;
        file.write_all(&content).map_err(CacheError::IOError)?;

        Ok(Action::Updated)
    }

    pub async fn ensure_async<E, F>(
        &self,
        source: impl Fn() -> F,
        output_path: impl AsRef<Path>,
    ) -> Result<Action, CacheError<E>>
    where
        F: Future<Output = Result<Vec<u8>, E>>, // E: Error + Send + Sync + 'static,
    {
        let file_path = self.base_dir.join(output_path);

        if file_path.exists() {
            return Ok(Action::Skipped);
        };

        let content = source().await.map_err(CacheError::SourceError)?;

        let mut file = File::create(&file_path).map_err(CacheError::IOError)?;
        file.write_all(&content).map_err(CacheError::IOError)?;

        Ok(Action::Updated)
    }
}
