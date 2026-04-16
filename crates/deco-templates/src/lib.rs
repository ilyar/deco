mod apply;
mod metadata;
mod model;

pub use apply::{apply_template, copy_directory_tree};
pub use metadata::{inspect_template_manifest_path, inspect_template_metadata};
pub use model::{
    TemplateApplyResult, TemplateCopyEntry, TemplateManifestDocument, TemplateManifestSummary,
    TemplatesMetadataResult, TemplatesScanMode,
};
