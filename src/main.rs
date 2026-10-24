mod builder;
mod cli;
mod codegen;
mod fs_layout;
mod rea;
mod triage;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use colored::*;
use fs_layout::OutputLayout;
use rea::ReaClient;
use std::path::Path;
use triage::TriageAnalyzer;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze { input, output } => {
            println!("{}", "=== BISMUTH: ANÁLISIS DE BINARIO CON REA ===".bold().cyan());
            run_analysis_pipeline(&input, &output)?;
        }
        Commands::Rebuild { input, output } => {
            println!("{}", "=== BISMUTH: RECONSTRUCCIÓN EN RUST ===".bold().green());
            let (layout, triage) = run_analysis_pipeline(&input, &output)?;
            run_rebuild_pipeline(&layout, &triage)?;
        }
        Commands::Full { input, output } => {
            println!("{}", "=== BISMUTH: PIPELINE COMPLETO DE MODERNIZACIÓN ===".bold().magenta());
            let (layout, triage) = run_analysis_pipeline(&input, &output)?;
            run_rebuild_pipeline(&layout, &triage)?;
        }
    }

    Ok(())
}

fn run_analysis_pipeline(input: &Path, output: &Path) -> Result<(OutputLayout, triage::TriageResult)> {
    println!("{} Objetivo: {}", "[1/4]".bold().blue(), input.display());
    println!("{} Directorio de salida: {}", "[2/4]".bold().blue(), output.display());

    let layout = OutputLayout::new(output)?;
    let client = ReaClient::new();

    println!("{} Invocando REA para extracción de importaciones PE...", "[*]".yellow());
    let imports = client.inspect_pe_imports(input)?;
    layout.write_analysis_file("dependencies.json", &serde_json::to_string_pretty(&imports)?)?;

    println!("{} Invocando REA para árbol de recursos...", "[*]".yellow());
    let resources = client.inspect_pe_resources(input)?;
    layout.write_analysis_file("resources_tree.json", &serde_json::to_string_pretty(&resources)?)?;

    println!("{} Invocando REA para verificación Authenticode...", "[*]".yellow());
    let certificates = client.inspect_pe_certificates(input)?;
    layout.write_analysis_file("certificates.json", &serde_json::to_string_pretty(&certificates)?)?;

    println!("{} Invocando REA para inspección de contenedor/empaquetador...", "[*]".yellow());
    let artifact = client.inspect_artifact(input).ok();
    if let Some(ref art) = artifact {
        layout.write_metadata_file("package_inventory.json", &serde_json::to_string_pretty(art)?)?;
    }

    println!("{} Clasificando binario y generando informe...", "[3/4]".bold().blue());
    let triage = TriageAnalyzer::analyze(
        &input.to_string_lossy(),
        &imports,
        &resources,
        &certificates,
        artifact.as_ref(),
    );

    layout.write_analysis_file("executable_info.json", &serde_json::to_string_pretty(&triage)?)?;

    println!("{}", "\n--- Resumen de Clasificación ---".bold());
    println!("  • Tecnología: {:?}", triage.technology);
    println!("  • Arquitectura: {}", triage.architecture);
    println!("  • Subsistema: {}", if triage.is_gui { "GUI (Windows)" } else { "Consola" });
    println!("  • Nivel de confianza: {:.1}%", triage.confidence * 100.0);
    for note in &triage.notes {
        println!("  • Nota: {}", note.dimmed());
    }

    println!("\n{} Análisis completado con éxito.", "[✓]".bold().green());
    Ok((layout, triage))
}

fn run_rebuild_pipeline(layout: &OutputLayout, triage: &triage::TriageResult) -> Result<()> {
    println!("\n{} Sintetizando proyecto Cargo en: {}", "[4/4]".bold().blue(), layout.reconstructed_dir.display());
    let codegen = codegen::CodeGenerator::new(layout, triage);
    codegen.generate_project()?;

    println!("{} Compilando y validando nuevo ejecutable...", "[*]".yellow());
    let builder = builder::ProjectBuilder::new(layout, triage);
    let success = builder.build_and_validate()?;

    if success {
        println!("\n{}", "🎉 ¡RECONSTRUCCIÓN Y COMPILACIÓN EXITOSA!".bold().green());
        println!("Nuevo binario generado en: {}", layout.build_dir.join("reconstructed.exe").display().to_string().bold());
        println!("Reporte de validación en: {}", layout.validation_dir.join("compatibility_report.md").display().to_string().dimmed());
    } else {
        println!("\n{}", "⚠️ La compilación encontró observaciones. Revisa el reporte de validación.".bold().yellow());
    }

    Ok(())
}
