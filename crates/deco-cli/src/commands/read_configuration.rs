use std::env;

use crate::cli::ReadConfigurationArgs;
use deco_config::ResolvedReadConfiguration;
use deco_core_model::DecoError;

pub fn run(args: ReadConfigurationArgs) -> Result<ResolvedReadConfiguration, DecoError> {
    let current_dir = env::current_dir().map_err(|error| {
        DecoError::new(
            deco_core_model::ErrorCategory::Internal,
            "failed to determine current working directory",
        )
        .with_details(error.to_string())
    })?;

    deco_config::resolve_read_configuration(
        &current_dir,
        args.workspace_folder.as_deref(),
        args.config.as_deref(),
        args.include_merged_configuration,
    )
}
