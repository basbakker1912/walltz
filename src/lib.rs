mod cli;
pub mod config;
pub mod image_supplier;
pub use cli::Program;

lazy_static::lazy_static! {
    pub static ref BASEDIRECTORIES: xdg::BaseDirectories = xdg::BaseDirectories::with_prefix("wallpaper_manager").expect("Failed to get base directories, is XDG installed?");
}
