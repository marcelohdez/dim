mod dim;
mod opts;
mod surface;

pub mod buffer;

pub use dim::DimData;
pub use opts::DimOpts;
pub use surface::DimSurface;

pub mod consts {
    pub const DEFAULT_DURATION: u64 = 30;
    pub const DEFAULT_ALPHA: f32 = 0.5;
    pub const DEFAULT_FADE: f32 = 0.5;

    pub const CONFIG_FILENAME: &str = "config.toml";
}
