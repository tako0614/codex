use dirs::home_dir;
use std::path::PathBuf;

/// This was copied from codex-core but codex-core depends on this crate.
/// TODO: move this to a shared crate lower in the dependency tree.
///
///
/// Returns the path to the Tacodex configuration directory, which can be
/// specified by the `TACODEX_HOME` environment variable. If not set, defaults
/// to `~/.tacodex`.
///
/// - If `TACODEX_HOME` is set, the value will be canonicalized and this
///   function will Err if the path does not exist.
/// - If not set, this function creates the directory if it doesn't exist.
pub(crate) fn find_codex_home() -> std::io::Result<PathBuf> {
    // Honor the `TACODEX_HOME` environment variable when it is set to allow users
    // (and tests) to override the default location.
    if let Ok(val) = std::env::var("TACODEX_HOME")
        && !val.is_empty()
    {
        return PathBuf::from(val).canonicalize();
    }

    let mut p = home_dir().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not find home directory",
        )
    })?;
    p.push(".tacodex");

    // Auto-create the directory if it doesn't exist
    if !p.exists() {
        std::fs::create_dir_all(&p)?;
    }

    Ok(p)
}
