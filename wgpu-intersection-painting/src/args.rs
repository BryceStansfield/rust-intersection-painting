use std::path::PathBuf;

use clap::{
    Args,
    Parser,
    Subcommand
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
    /// Input Path
    pub input: PathBuf,

    /// Output Path
    pub output: PathBuf,

    #[clap(subcommand)]
    pub generator: Generator,
}

#[derive(Debug, Args)]
pub struct StaticCommand{
    /// Stencil Path
    pub stencil: PathBuf,

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
}

#[derive(Debug, Args)]
pub struct SquareGridCommand{
    pub side_length: usize
}

#[derive(Debug, Args)]
pub struct CircleGridCommand{
    pub radius: usize
}