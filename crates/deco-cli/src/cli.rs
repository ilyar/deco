use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "deco")]
#[command(version, about = "Rust-first dev container CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    ReadConfiguration(ReadConfigurationArgs),
    Build(TargetArgs),
    Up(TargetArgs),
    Exec(ExecArgs),
    RunUserCommands(RunUserCommandsArgs),
    SetUp(SetUpArgs),
    Features(FeaturesArgs),
    Templates(TemplatesArgs),
    Outdated(LockfileArgs),
    Upgrade(UpgradeArgs),
}

#[derive(Debug, Clone, Args)]
pub struct ReadConfigurationArgs {
    #[arg(long)]
    pub workspace_folder: Option<PathBuf>,
    #[arg(long)]
    pub config: Option<PathBuf>,
    #[arg(long)]
    pub include_merged_configuration: bool,
}

#[derive(Debug, Clone, Args)]
pub struct TargetArgs {
    #[arg(long)]
    pub workspace_folder: Option<PathBuf>,
    #[arg(long)]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct ExecArgs {
    #[arg(long)]
    pub container_id: Option<String>,
    #[arg(long)]
    pub workspace_folder: Option<PathBuf>,
    #[arg(long)]
    pub config: Option<PathBuf>,
    #[arg(long)]
    pub user: Option<String>,
    #[arg(long)]
    pub workdir: Option<String>,
    #[arg(trailing_var_arg = true, required = true)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Args)]
pub struct RunUserCommandsArgs {
    #[arg(long)]
    pub container_id: Option<String>,
    #[arg(long)]
    pub workspace_folder: Option<PathBuf>,
    #[arg(long)]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct SetUpArgs {
    #[command(flatten)]
    pub target: TargetArgs,
}

#[derive(Debug, Clone, Args)]
pub struct FeaturesArgs {
    #[command(subcommand)]
    pub command: Option<FeaturesCommand>,
    #[command(flatten)]
    pub inspect: FeaturesInspectArgs,
}

#[derive(Debug, Clone, Subcommand)]
pub enum FeaturesCommand {
    ResolveDependencies(FeaturesInspectArgs),
    Test(FeaturesTestArgs),
}

#[derive(Debug, Clone, Args)]
pub struct FeaturesInspectArgs {
    #[arg(long)]
    pub manifest_dir: Option<PathBuf>,
    #[arg(long)]
    pub workspace_folder: Option<PathBuf>,
    #[arg(long)]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct FeaturesTestArgs {
    #[arg(long)]
    pub manifest_dir: Option<PathBuf>,
    #[arg(long, short = 'p')]
    pub project_folder: Option<PathBuf>,
    #[arg()]
    pub target: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct TemplatesArgs {
    #[command(subcommand)]
    pub command: TemplatesCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum TemplatesCommand {
    Metadata(TemplatesMetadataArgs),
    Apply(TemplatesApplyArgs),
}

#[derive(Debug, Clone, Args)]
pub struct TemplatesMetadataArgs {
    #[arg(long)]
    pub manifest_path: Option<PathBuf>,
    #[arg(long)]
    pub template_id: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct TemplatesApplyArgs {
    #[arg(long)]
    pub manifest_path: Option<PathBuf>,
    #[arg(long)]
    pub template_id: Option<PathBuf>,
    #[arg(long)]
    pub workspace_folder: Option<PathBuf>,
    #[arg(long)]
    pub target_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct LockfileArgs {
    #[arg(long)]
    pub lockfile: Option<PathBuf>,
    #[arg(long)]
    pub workspace_folder: Option<PathBuf>,
    #[arg(long)]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct UpgradeArgs {
    #[command(flatten)]
    pub lockfile: LockfileArgs,
    #[arg(long)]
    pub dry_run: bool,
}
