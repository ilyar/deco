mod read_configuration;

pub use read_configuration::{
    BuildSpec, ComposeSpec, DevcontainerConfigKind, NormalizedDevcontainerConfig,
    ResolvedReadConfiguration, resolve_read_configuration,
};
