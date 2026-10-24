mod builder;
mod cli;
mod codegen;
mod events;
mod fs_layout;
mod local_pe;
mod rea;
mod triage;
mod tui;
mod web;

use anyhow::{Result, ensure};
use clap::Parser;
use cli::{Cli, Commands};
use colored::*;
use events::{PipelineStage, SessionGuard, StateManager, start_host_monitor};
use fs_layout::OutputLayout;
use rea::ReaClient;
use std::path::Path;
use triage::TriageAnalyzer;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let state_mgr = StateManager::new();
    let _session_guard = SessionGuard::new(state_mgr.clone());
    start_host_monitor(state_mgr.clone());

    match cli.command {
        Commands::Analyze {
            input,
            output,
            web,
            port,
        } => {
            let web_thread = if web {
                Some(start_dashboard(state_mgr.clone(), port))
            } else {
                None
            };
            println!(
                "{}",
                "=== BISMUTH: ANÁLISIS DE BINARIO CON REA ===".bold().cyan()
            );
            let (_layout, _triage) = run_analysis_pipeline(&input, &output, &state_mgr)?;
            state_mgr.finish_session();
            if let Some(handle) = web_thread {
                let _ = handle.join();
            }
        }
        Commands::Rebuild { input, output } => {
            println!(
                "{}",
                "=== BISMUTH: RECONSTRUCCIÓN EN RUST ===".bold().green()
            );
            let (layout, triage) = run_analysis_pipeline(&input, &output, &state_mgr)?;
            run_rebuild_pipeline(&layout, &triage, &state_mgr)?;
            state_mgr.finish_session();
        }
        Commands::Full {
            input,
            output,
            web,
            port,
        } => {
            let web_thread = if web {
                Some(start_dashboard(state_mgr.clone(), port))
            } else {
                None
            };
            println!(
                "{}",
                "=== BISMUTH: PIPELINE COMPLETO DE MODERNIZACIÓN ==="
                    .bold()
                    .magenta()
            );
            let (layout, triage) = run_analysis_pipeline(&input, &output, &state_mgr)?;
            run_rebuild_pipeline(&layout, &triage, &state_mgr)?;
            state_mgr.finish_session();
            if let Some(handle) = web_thread {
                let _ = handle.join();
            }
        }
        Commands::Tui { input } => {
            let triage_opt = if let Some(ref target) = input {
                state_mgr.log(
                    "INFO",
                    &format!("Cargando análisis para TUI: {}", target.display()),
                );
                let client = ReaClient::new();
                let imports = client.inspect_pe_imports(target).unwrap_or_default();
                let resources = client.inspect_pe_resources(target).unwrap_or_default();
                let certs = client.inspect_pe_certificates(target).unwrap_or_default();
                let artifact = client.inspect_artifact(target).ok();
                let tr = TriageAnalyzer::analyze(
                    &target.to_string_lossy(),
                    &imports,
                    &resources,
                    &certs,
                    artifact.as_ref(),
                );
                state_mgr.set_target(
                    &target.to_string_lossy(),
                    &format!("{:?}", tr.technology),
                    &tr.architecture,
                    tr.imported_apis.iter().map(|a| a.functions.len()).sum(),
                );
                state_mgr.set_stage(PipelineStage::Completed, 100);
                state_mgr.finish_session();
                Some(tr)
            } else {
                None
            };
            tui::run_tui(state_mgr, triage_opt)?;
        }
        Commands::Web { input, port } => {
            println!(
                "{}",
                format!("=== BISMUTH LIVE DASHBOARD (http://localhost:{}) ===", port)
                    .bold()
                    .cyan()
            );
            let triage_opt = if let Some(ref target) = input {
                state_mgr.log(
                    "INFO",
                    &format!(
                        "Cargando análisis previo para Dashboard: {}",
                        target.display()
                    ),
                );
                let client = ReaClient::new();
                let imports = client.inspect_pe_imports(target).unwrap_or_default();
                let resources = client.inspect_pe_resources(target).unwrap_or_default();
                let certs = client.inspect_pe_certificates(target).unwrap_or_default();
                let artifact = client.inspect_artifact(target).ok();
                let tr = TriageAnalyzer::analyze(
                    &target.to_string_lossy(),
                    &imports,
                    &resources,
                    &certs,
                    artifact.as_ref(),
                );
                state_mgr.set_target(
                    &target.to_string_lossy(),
                    &format!("{:?}", tr.technology),
                    &tr.architecture,
                    tr.imported_apis.iter().map(|a| a.functions.len()).sum(),
                );
                state_mgr.set_stage(PipelineStage::Completed, 100);
                state_mgr.finish_session();
                Some(tr)
            } else {
                None
            };
            web::start_web_server(state_mgr, triage_opt, port)?;
        }
    }

    Ok(())
}

fn run_analysis_pipeline(
    input: &Path,
    output: &Path,
    state_mgr: &StateManager,
) -> Result<(OutputLayout, triage::TriageResult)> {
    ensure!(
        input.is_file(),
        "El ejecutable objetivo no existe o no es un archivo: {}",
        input.display()
    );
    state_mgr.set_stage(PipelineStage::Triage, 10);
    state_mgr.log(
        "INFO",
        &format!("Iniciando análisis del objetivo: {}", input.display()),
    );

    println!("{} Objetivo: {}", "[1/4]".bold().blue(), input.display());
    println!(
        "{} Directorio de salida: {}",
        "[2/4]".bold().blue(),
        output.display()
    );

    let layout = OutputLayout::new(output)?;
    let client = ReaClient::new();

    state_mgr.set_stage(PipelineStage::ReaExtraction, 25);
    state_mgr.log("REA", "Consultando inspect-pe-imports...");
    println!(
        "{} Invocando REA para extracción de importaciones PE...",
        "[*]".yellow()
    );
    let imports = match client.inspect_pe_imports(input) {
        Ok(value) => {
            state_mgr.log("REA", "Inventario de APIs obtenido mediante inspect.");
            value
        }
        Err(error) => {
            let message = format!(
                "Importaciones no disponibles; se continúa con evidencia parcial: {error:#}"
            );
            println!("{} {}", "[!]".yellow(), message);
            state_mgr.log("WARN", &message);
            serde_json::json!({"descriptors": [], "availability": "unavailable"})
        }
    };
    layout.write_analysis_file(
        "dependencies.json",
        &serde_json::to_string_pretty(&imports)?,
    )?;
    state_mgr.log("REA", "Tabla de importaciones (IAT) extraída con éxito.");

    state_mgr.set_stage(PipelineStage::ReaExtraction, 40);
    state_mgr.log("REA", "Consultando inspect-pe-resources...");
    println!("{} Invocando REA para árbol de recursos...", "[*]".yellow());
    let resources = client.inspect_pe_resources(input)?;
    layout.write_analysis_file(
        "resources_tree.json",
        &serde_json::to_string_pretty(&resources)?,
    )?;

    state_mgr.set_stage(PipelineStage::ReaExtraction, 50);
    state_mgr.log("REA", "Consultando inspect-pe-certificates...");
    println!(
        "{} Invocando REA para verificación Authenticode...",
        "[*]".yellow()
    );
    let certificates = match client.inspect_pe_certificates(input) {
        Ok(value) => value,
        Err(error) => {
            let message =
                format!("Certificados PE no disponibles en la CLI actual de REA: {error:#}");
            println!("{} {}", "[!]".yellow(), message);
            state_mgr.log("WARN", &message);
            serde_json::json!({"signers": [], "availability": "unavailable"})
        }
    };
    layout.write_analysis_file(
        "certificates.json",
        &serde_json::to_string_pretty(&certificates)?,
    )?;

    state_mgr.log("REA", "Consultando inspect-artifact...");
    println!(
        "{} Invocando REA para inspección de contenedor/empaquetador...",
        "[*]".yellow()
    );
    let artifact = client.inspect_artifact(input).ok();
    if let Some(ref art) = artifact {
        layout.write_metadata_file(
            "package_inventory.json",
            &serde_json::to_string_pretty(art)?,
        )?;
        state_mgr.log("REA", "Inventario de artefactos y contenedor generado.");
    }

    state_mgr.set_stage(PipelineStage::Triage, 65);
    println!(
        "{} Clasificando binario y generando informe...",
        "[3/4]".bold().blue()
    );
    let triage = TriageAnalyzer::analyze(
        &input.to_string_lossy(),
        &imports,
        &resources,
        &certificates,
        artifact.as_ref(),
    );

    let total_funcs: usize = triage.imported_apis.iter().map(|a| a.functions.len()).sum();
    state_mgr.set_target(
        &input.to_string_lossy(),
        &format!("{:?}", triage.technology),
        &triage.architecture,
        total_funcs,
    );
    state_mgr.set_triage(triage.clone());

    layout.write_analysis_file(
        "executable_info.json",
        &serde_json::to_string_pretty(&triage)?,
    )?;

    println!("{}", "\n--- Resumen de Clasificación ---".bold());
    println!("  • Tecnología: {:?}", triage.technology);
    println!("  • Arquitectura: {}", triage.architecture);
    println!(
        "  • Subsistema: {}",
        if triage.is_gui {
            "GUI (Windows)"
        } else {
            "Consola"
        }
    );
    println!("  • Nivel de confianza: {:.1}%", triage.confidence * 100.0);
    for note in &triage.notes {
        println!("  • Nota: {}", note.dimmed());
        state_mgr.log("WARN", note);
    }

    state_mgr.log("INFO", "Triage completado.");
    println!("\n{} Análisis completado con éxito.", "[✓]".bold().green());
    Ok((layout, triage))
}

fn start_dashboard(state_mgr: StateManager, port: u16) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        if let Err(error) = web::start_web_server(state_mgr, None, port) {
            eprintln!("Error del dashboard web: {error:#}");
        }
    })
}

fn run_rebuild_pipeline(
    layout: &OutputLayout,
    triage: &triage::TriageResult,
    state_mgr: &StateManager,
) -> Result<()> {
    state_mgr.set_stage(PipelineStage::CodeSynthesis, 75);
    state_mgr.log("BUILD", "Sintetizando proyecto Cargo en Rust...");

    println!(
        "\n{} Sintetizando proyecto Cargo en: {}",
        "[4/4]".bold().blue(),
        layout.reconstructed_dir.display()
    );
    let codegen = codegen::CodeGenerator::new(layout, triage);
    codegen.generate_project()?;
    state_mgr.log("BUILD", "Cargo.toml y src/main.rs generados.");

    state_mgr.set_stage(PipelineStage::CargoBuild, 85);
    state_mgr.log("BUILD", "Invocando cargo build --release...");
    println!(
        "{} Compilando y validando nuevo ejecutable...",
        "[*]".yellow()
    );
    let builder = builder::ProjectBuilder::new(layout, triage);
    let success = builder.build_and_validate()?;

    if success {
        state_mgr.set_stage(PipelineStage::Completed, 100);
        state_mgr.log("BUILD", "¡Reconstrucción y compilación exitosa!");
        println!(
            "\n{}",
            "🎉 ¡RECONSTRUCCIÓN Y COMPILACIÓN EXITOSA!".bold().green()
        );
        println!(
            "Nuevo binario generado en: {}",
            layout
                .build_dir
                .join("reconstructed.exe")
                .display()
                .to_string()
                .bold()
        );
        println!(
            "Reporte de validación en: {}",
            layout
                .validation_dir
                .join("compatibility_report.md")
                .display()
                .to_string()
                .dimmed()
        );
    } else {
        state_mgr.set_stage(PipelineStage::Failed, 85);
        state_mgr.log("ERROR", "La compilación de Cargo encontró observaciones.");
        println!(
            "\n{}",
            "⚠️ La compilación encontró observaciones. Revisa el reporte de validación."
                .bold()
                .yellow()
        );
    }

    Ok(())
}
