use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "bismuth",
    author = "Bismuth RE Modernization Engine",
    version = "0.1.0",
    about = "Reconstrucción automatizada de ejecutables Windows en Rust mediante REA"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Ejecuta el análisis y extracción de evidencias mediante REA
    Analyze {
        /// Ruta al ejecutable .exe de Windows a analizar
        #[arg(short, long)]
        input: PathBuf,

        /// Directorio de salida donde guardar los resultados (por defecto: ./output)
        #[arg(short, long, default_value = "./output")]
        output: PathBuf,
    },

    /// Sintetiza el proyecto en Rust y compila un nuevo ejecutable
    Rebuild {
        /// Ruta al ejecutable .exe de Windows
        #[arg(short, long)]
        input: PathBuf,

        /// Directorio de salida (por defecto: ./output)
        #[arg(short, long, default_value = "./output")]
        output: PathBuf,
    },

    /// Ejecuta el flujo completo: Análisis -> Extracción -> Reconstrucción -> Compilación -> Validación
    Full {
        /// Ruta al ejecutable .exe de Windows
        #[arg(short, long)]
        input: PathBuf,

        /// Directorio de salida (por defecto: ./output)
        #[arg(short, long, default_value = "./output")]
        output: PathBuf,
    },
}
