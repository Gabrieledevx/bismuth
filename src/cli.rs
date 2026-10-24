use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "bismuth",
    author = "Gabriel Medranda (@Gabrieledevx)",
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

        /// Inicia el dashboard web tras completar el análisis
        #[arg(long, default_value_t = false)]
        web: bool,

        /// Puerto del dashboard web cuando se usa --web
        #[arg(long, default_value_t = 3000)]
        port: u16,
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

        /// Inicia el dashboard web en segundo plano
        #[arg(long, default_value_t = false)]
        web: bool,

        /// Puerto del dashboard web cuando se usa --web
        #[arg(long, default_value_t = 3000)]
        port: u16,
    },

    /// Abre la consola interactiva en la terminal (TUI)
    Tui {
        /// Ruta opcional a un ejecutable para inspeccionar
        #[arg(short, long)]
        input: Option<PathBuf>,
    },

    /// Inicia el dashboard visual web en http://localhost:3000
    Web {
        /// Ruta al ejecutable a inspeccionar
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Puerto del servidor HTTP local
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
    },
}
