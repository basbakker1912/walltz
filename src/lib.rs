mod cli;
pub mod config;
pub mod image_supplier;
pub use cli::Program;
use config::GlobalConfig;
pub mod collections;
pub mod state;

lazy_static::lazy_static! {
    pub static ref BASEDIRECTORIES: xdg::BaseDirectories = xdg::BaseDirectories::with_prefix(env!("CARGO_PKG_NAME")).expect("Failed to get base directories, is XDG installed?");
    pub static ref CONFIG: GlobalConfig = GlobalConfig::read().expect("Failed to open config.");

}
