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

pub trait SourceAsync<E> {
    async fn get_async(&self) -> Result<Vec<u8>, E>;
}

pub trait Source<E> {
    fn get(&self) -> Result<Vec<u8>, E>;
}

// impl CacheSourceAsync for dyn Fn() -> dyn Future<Output = Result<Vec<u8>, anyhow::Error>> {
//     async fn get_async(&self) -> Result<Vec<u8>, anyhow::Error> {
//         (*self)()
//     }
// }

// impl<T: Send + Sync + 'static> CacheSourceAsync for T
// where
//     T: Fn() -> Result<Vec<u8>, anyhow::Error> + Send + Sync + 'static,
// {
//     async fn get_async(&self) -> Result<Vec<u8>, anyhow::Error> {
//         self()
//     }
// }
impl<E, F, Fut> SourceAsync<E> for F
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<Vec<u8>, E>>,
{
    async fn get_async(&self) -> Result<Vec<u8>, E> {
        self().await
    }
}

impl<E, F> Source<E> for F
where
    F: Fn() -> Result<Vec<u8>, E>,
{
    fn get(&self) -> Result<Vec<u8>, E> {
        self()
    }
}

// impl<E> CacheSource<E> for dyn Fn() -> Result<Vec<u8>, E> {
//     fn get(&self) -> Result<Vec<u8>, E> {
//         self()
//     }
// }

#[derive(thiserror::Error, Debug)]
pub enum CacheError<E>
where
// E1: Error,
{
    #[error("File exsists")]
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

    pub fn ensure<S, E>(&self, source: S, output_path: &Path) -> Result<Action, CacheError<E>>
    where
        S: Source<E>,
        // E: Error + Send + Sync + 'static,
    {
        let file_path = self.base_dir.join(output_path);

        if file_path.exists() {
            return Ok(Action::Skipped);
        }

        let content = source.get().map_err(CacheError::SourceError)?;

        let mut file = File::create(&file_path).map_err(CacheError::IOError)?;
        file.write_all(&content).map_err(CacheError::IOError)?;

        Ok(Action::Updated)
    }

    pub async fn ensure_async<S, E>(
        &self,
        source: S,
        output_path: impl AsRef<Path>,
    ) -> Result<Action, CacheError<E>>
    where
        S: SourceAsync<E>,
        // E: Error + Send + Sync + 'static,
    {
        let file_path = self.base_dir.join(output_path);

        if file_path.exists() {
            return Ok(Action::Skipped);
        };

        let content = source.get_async().await.map_err(CacheError::SourceError)?;

        let mut file = File::create(&file_path).map_err(CacheError::IOError)?;
        file.write_all(&content).map_err(CacheError::IOError)?;

        Ok(Action::Updated)
    }
}
