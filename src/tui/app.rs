use crate::events::{PipelineStage, StateManager};
use crate::triage::TriageResult;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
};
use std::io;
use std::time::Duration;

pub fn run_tui(state_mgr: StateManager, triage: Option<TriageResult>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut selected_api_index = 0;

    loop {
        let snapshot = state_mgr.snapshot();

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Header
                    Constraint::Min(10),   // Cuerpo principal
                    Constraint::Length(3), // Barra de estado / Footer
                ])
                .split(f.area());

            // 1. HEADER
            let title = Line::from(vec![
                Span::styled(
                    " ⚗️ BISMUTH RE MODERNIZER ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" | "),
                Span::styled(
                    format!(
                        "Target: {}",
                        snapshot.target_file.as_deref().unwrap_or("Ninguno")
                    ),
                    Style::default().fg(Color::White),
                ),
                Span::raw(" | "),
                Span::styled(
                    format!(
                        "Arch: {}",
                        snapshot.architecture.as_deref().unwrap_or("N/A")
                    ),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(" | "),
                Span::styled(
                    format!("Tech: {}", snapshot.technology.as_deref().unwrap_or("N/A")),
                    Style::default().fg(Color::Green),
                ),
            ]);
            let header_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            f.render_widget(Paragraph::new(title).block(header_block), chunks[0]);

            // 2. CUERPO PRINCIPAL (Izquierda: Inspector PE | Derecha: Estado + Consola)
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
                .split(chunks[1]);

            // 2.1 PANEL IZQUIERDO: INSPECTOR DE BINARIO Y APIS
            let mut items = Vec::new();
            if let Some(ref tr) = triage {
                items.push(ListItem::new(Line::from(vec![
                    Span::styled("Tecnología: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{:?}", tr.technology),
                        Style::default()
                            .fg(Color::LightGreen)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])));
                items.push(ListItem::new(Line::from(vec![
                    Span::styled("Subsistema: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        if tr.is_gui {
                            "GUI (Windows)"
                        } else {
                            "Consola"
                        },
                        Style::default().fg(Color::LightYellow),
                    ),
                ])));
                items.push(ListItem::new(Line::from(vec![
                    Span::styled("Confianza: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{:.1}%", tr.confidence * 100.0),
                        Style::default().fg(Color::Cyan),
                    ),
                ])));
                items.push(ListItem::new(Span::raw(" ")));
                items.push(ListItem::new(Span::styled(
                    "--- Módulos Win32 / DLLs Importadas ---",
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                )));

                for (idx, api) in tr.imported_apis.iter().enumerate() {
                    let prefix = if idx == selected_api_index {
                        "> "
                    } else {
                        "  "
                    };
                    let style = if idx == selected_api_index {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    items.push(ListItem::new(Span::styled(
                        format!("{}{} ({} funciones)", prefix, api.dll, api.functions.len()),
                        style,
                    )));
                }
            } else {
                items.push(ListItem::new(
                    "Sin análisis cargado aún. Presiona [F] para iniciar.",
                ));
            }

            let inspector_block = Block::default()
                .title(" 🔍 INSPECTOR DE BINARIO (PE & REA) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue));
            f.render_widget(List::new(items).block(inspector_block), main_chunks[0]);

            // 2.2 PANEL DERECHO (Arriba: Estados | Abajo: Consola)
            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(8), Constraint::Min(8)])
                .split(main_chunks[1]);

            // ESTADO DEL PIPELINE
            let stage_label = match snapshot.stage {
                PipelineStage::Idle => "En Espera",
                PipelineStage::Triage => "1. Triage & Cabeceras PE",
                PipelineStage::ReaExtraction => "2. Extracción de Evidencias REA",
                PipelineStage::CodeSynthesis => "3. Síntesis Semántica en Rust",
                PipelineStage::CargoBuild => "4. Compilación con Cargo",
                PipelineStage::Validation => "5. Validación de Fidelidad",
                PipelineStage::Completed => "✅ Proceso Finalizado con Éxito",
                PipelineStage::Failed => "❌ Falló el Pipeline",
            };

            let gauge = Gauge::default()
                .block(
                    Block::default()
                        .title(" ⏱️ ESTADO DEL PIPELINE ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Green)),
                )
                .gauge_style(Style::default().fg(Color::Cyan).bg(Color::DarkGray))
                .percent(snapshot.progress_pct as u16)
                .label(format!("{} ({}%)", stage_label, snapshot.progress_pct));
            f.render_widget(gauge, right_chunks[0]);

            // CONSOLA & LOGS EN VIVO
            let log_items: Vec<ListItem> = snapshot
                .logs
                .iter()
                .rev()
                .take(30)
                .map(|log| {
                    let level_style = match log.level.as_str() {
                        "INFO" => Style::default().fg(Color::Cyan),
                        "REA" => Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                        "WARN" => Style::default().fg(Color::Yellow),
                        "ERROR" => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        "BUILD" => Style::default().fg(Color::Green),
                        _ => Style::default().fg(Color::White),
                    };

                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("{} ", log.timestamp),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(format!("[{:5}] ", log.level), level_style),
                        Span::raw(log.message.clone()),
                    ]))
                })
                .collect();

            let console_block = Block::default()
                .title(" 📜 CONSOLA DE EVENTOS & LOGS ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));
            f.render_widget(List::new(log_items).block(console_block), right_chunks[1]);

            // 3. FOOTER
            let footer_text = Line::from(vec![
                Span::styled(
                    " [↑/↓] ",
                    Style::default().fg(Color::Black).bg(Color::White),
                ),
                Span::raw(" Navegar APIs  "),
                Span::styled(" [F] ", Style::default().fg(Color::Black).bg(Color::Green)),
                Span::raw(" Ejecutar Reconstrucción  "),
                Span::styled(" [Q] ", Style::default().fg(Color::Black).bg(Color::Red)),
                Span::raw(" Salir "),
                Span::raw("  |  "),
                Span::styled(
                    format!(
                        "Sesión {} · {} · CPU {:.0}%",
                        snapshot.session.id,
                        format_duration(snapshot.session.elapsed_ms),
                        snapshot.session.host.cpu_percent
                    ),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            let footer_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray));
            f.render_widget(Paragraph::new(footer_text).block(footer_block), chunks[2]);
        })?;

        // Manejo de eventos de teclado (no bloqueante largo para refrescar logs)
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => break,
                    KeyCode::Up => {
                        if selected_api_index > 0 {
                            selected_api_index -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if let Some(ref tr) = triage {
                            if selected_api_index + 1 < tr.imported_apis.len() {
                                selected_api_index += 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn format_duration(elapsed_ms: u64) -> String {
    let seconds = elapsed_ms / 1000;
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        (seconds / 60) % 60,
        seconds % 60
    )
}
