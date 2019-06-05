pub use self::loader::Loader;
pub use self::mysql_loader::MysqlLoader;
#[cfg(feature = "sqlite")]
pub use self::sqlite_loader::SqliteLoader;

mod loader;
mod mysql_loader;
#[cfg(feature = "sqlite")]
mod sqlite_loader;
