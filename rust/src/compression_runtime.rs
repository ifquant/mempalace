use crate::VERSION;
use crate::compress::{CompressSummaryContext, CompressionRun};
use crate::config::AppConfig;
use crate::dialect::Dialect;
use crate::embed::EmbeddingProvider;
use crate::error::Result;
use crate::model::CompressSummary;
use crate::storage::sqlite::SqliteStore;

pub struct CompressionRuntime<'a> {
    pub config: &'a AppConfig,
    pub embedder: &'a dyn EmbeddingProvider,
}

impl<'a> CompressionRuntime<'a> {
    fn open_sqlite(&self) -> Result<SqliteStore> {
        self.config.ensure_dirs()?;
        let sqlite = SqliteStore::open(&self.config.sqlite_path())?;
        sqlite.init_schema()?;
        sqlite.ensure_embedding_profile(self.embedder.profile())?;
        Ok(sqlite)
    }

    pub fn compress(&self, wing: Option<&str>, dry_run: bool) -> Result<CompressSummary> {
        let dialect = Dialect;
        let mut sqlite = self.open_sqlite()?;
        let drawers = sqlite.list_drawers(wing)?;
        let run = CompressionRun::from_drawers(drawers, &dialect);

        if !dry_run {
            sqlite.replace_compressed_drawers(wing, &run.entries)?;
        }

        Ok(run.into_summary(CompressSummaryContext {
            palace_path: self.config.palace_path.display().to_string(),
            sqlite_path: self.config.sqlite_path().display().to_string(),
            version: VERSION.to_string(),
            wing: wing.map(ToOwned::to_owned),
            dry_run,
        }))
    }
}
