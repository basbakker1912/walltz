mod cli;
pub mod config;
pub mod image;
pub use cli::Program;
use config::GlobalConfig;
use image::cache::ImageCache;
pub mod category;
pub mod collections;
pub mod finder;
pub mod state;

lazy_static::lazy_static! {
    pub static ref BASEDIRECTORIES: xdg::BaseDirectories = xdg::BaseDirectories::with_prefix(env!("CARGO_PKG_NAME")).expect("Failed to get base directories, is XDG installed?");
    pub static ref CONFIG: GlobalConfig = GlobalConfig::read().expect("Failed to open config.");
    pub static ref IMAGECACHE: ImageCache = {
        let cache = ImageCache::open();
        match cache.cleanup_cache() {
            Ok(_) => {},
            Err(err) => println!("Failed to clean the cache: {:?}", err)
        }
        cache
    };
}
