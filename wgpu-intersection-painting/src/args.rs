use std::path::PathBuf;

use clap::{
    Args,
    Parser,
    Subcommand,
};

#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct Arguments {
    /// Which mode should I run in? `dynamic` for a dynamically generated stencil, and `stencil` for a static one
    #[clap(subcommand)]
    pub command_type: GeneratorType
}


#[derive(Debug, Subcommand)]
pub enum GeneratorType{
    Dynamic(DynamicCommand),
    Static(StaticCommand),
    GenerateStencil(GenerateStencilCommand)
}

#[derive(Debug, Args)]
pub struct DynamicCommand{
    /// Input Path, folder or file
    pub input: PathBuf,

    /// Output Path, folder or file
    pub output: PathBuf,

    /// Alpha averaging enabled?
    #[arg(short, long)]
    pub alpha_averaging: bool,

    #[clap(subcommand)]
    pub generator: Generator,
}

#[derive(Debug, Args)]
pub struct StaticCommand{
    /// Stencil Path
    pub stencil: PathBuf,

    /// Alpha averaging enabled?
    #[arg(short, long)]
    pub alpha_averaging: bool,

    /// Input Path
    pub input: PathBuf,

    /// Output Path
    pub output: PathBuf
}

#[derive(Debug, Args)]
pub struct GenerateStencilCommand{
    /// Width
    pub width: u32,

    /// Height
    pub height: u32,

    /// Output Path
    pub output: PathBuf,

    #[clap(subcommand)]
    pub generator: Generator,
}

#[derive(Debug, Subcommand)]
pub enum Generator{
    SquareGrid(SquareGridCommand),
    CircleGrid(CircleGridCommand),
    CrossGrid(CrossGridCommand),
    MaskGrid(MaskGridCommand)
}

#[derive(Debug, Args)]
pub struct SquareGridCommand{
    pub side_length: u32
}

#[derive(Debug, Args)]
pub struct CircleGridCommand{
    pub radius: u32
}

#[derive(Debug, Args)]
pub struct CrossGridCommand{
    pub cross_intersection_width: u32
}

#[derive(Debug, Args)]
pub struct MaskGridCommand{
    pub mask_folder: PathBuf
}