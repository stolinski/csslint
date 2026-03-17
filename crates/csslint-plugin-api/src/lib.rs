#![forbid(unsafe_code)]

pub trait PluginApiVersion {
    fn api_version() -> &'static str;
}
