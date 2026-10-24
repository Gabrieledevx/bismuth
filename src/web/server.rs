use crate::events::StateManager;
use crate::triage::TriageResult;
use anyhow::Result;
use std::sync::{Arc, Mutex};
use tiny_http::{Header, Response, Server};

pub fn start_web_server(
    state_mgr: StateManager,
    triage: Option<TriageResult>,
    port: u16,
) -> Result<()> {
    let addr = format!("127.0.0.1:{}", port);
    let server = Server::http(&addr)
        .map_err(|e| anyhow::anyhow!("Error iniciando servidor en {}: {}", addr, e))?;
    let triage_arc = Arc::new(Mutex::new(triage));

    println!("🌐 Servidor Web de Bismuth iniciado en: http://{}", addr);

    // Intentar abrir el navegador automáticamente en Windows
    #[cfg(windows)]
    let _ = std::process::Command::new("powershell")
        .args(["-Command", &format!("Start-Process 'http://{}'", addr)])
        .spawn();

    for request in server.incoming_requests() {
        let url = request.url().to_string();

        if url == "/" {
            let html = get_dashboard_html();
            let header =
                Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap();
            let mut response = Response::from_string(html);
            response.add_header(header);
            let _ = request.respond(response);
        } else if url == "/health" {
            let response = Response::from_string("ok").with_header(
                Header::from_bytes(&b"Content-Type"[..], &b"text/plain; charset=utf-8"[..])
                    .unwrap(),
            );
            let _ = request.respond(response);
        } else if url.starts_with("/api/status") {
            let snap = state_mgr.snapshot();
            let tr_guard = triage_arc.lock().unwrap();
            let triage = tr_guard.clone().or_else(|| snap.triage.clone());
            let json = serde_json::json!({
                "state": snap,
                "triage": triage
            });

            let header =
                Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
            let mut response = Response::from_string(json.to_string());
            response.add_header(header);
            let _ = request.respond(response);
        } else {
            let response = Response::from_string("404 Not Found").with_status_code(404);
            let _ = request.respond(response);
        }
    }

    Ok(())
}

fn get_dashboard_html() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="es">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Bismuth | Modernización & Reconstrucción de Binarios PE con REA</title>
  <meta name="description" content="Dashboard en tiempo real para inspección de ejecutables Windows, seguimiento de evidencias REA y reconstrucción en Rust.">
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500;600&display=swap" rel="stylesheet">
  <style>
    :root {
      color-scheme: dark;
      --bg-base: #0b0f19;
      --bg-surface: rgba(17, 24, 39, 0.75);
      --bg-elevated: rgba(31, 41, 55, 0.8);
      --border-subtle: rgba(255, 255, 255, 0.08);
      --border-glow: rgba(34, 211, 238, 0.25);
      --accent-cyan: #22d3ee;
      --accent-emerald: #10b981;
      --accent-amber: #f59e0b;
      --accent-purple: #a855f7;
      --accent-red: #ef4444;
      --text-main: #f9fafb;
      --text-muted: #9ca3af;
      --font-ui: 'Inter', system-ui, sans-serif;
      --font-mono: 'JetBrains Mono', monospace;
    }

    * { box-sizing: border-box; margin: 0; padding: 0; }
    .sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0,0,0,0); white-space: nowrap; border: 0; }
    body {
      background: radial-gradient(circle at 10% 10%, #111827 0%, var(--bg-base) 90%);
      color: var(--text-main);
      font-family: var(--font-ui);
      min-height: 100vh;
      display: flex;
      flex-direction: column;
    }

    header {
      background: rgba(11, 15, 25, 0.85);
      backdrop-filter: blur(12px);
      border-bottom: 1px solid var(--border-subtle);
      padding: 1rem 2rem;
      display: flex;
      align-items: center;
      justify-content: space-between;
      position: sticky;
      top: 0;
      z-index: 50;
    }

    .brand {
      display: flex;
      align-items: center;
      gap: 0.75rem;
    }

    .brand-icon {
      font-size: 1.75rem;
      background: linear-gradient(135deg, #22d3ee, #a855f7);
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
    }

    .brand-title {
      font-size: 1.25rem;
      font-weight: 700;
      letter-spacing: -0.025em;
    }

    .brand-tag {
      font-size: 0.7rem;
      background: rgba(34, 211, 238, 0.12);
      color: var(--accent-cyan);
      padding: 0.2rem 0.5rem;
      border-radius: 9999px;
      border: 1px solid rgba(34, 211, 238, 0.3);
      font-weight: 600;
      text-transform: uppercase;
    }

    .target-badge-group {
      display: flex;
      align-items: center;
      gap: 1rem;
    }

    .badge {
      display: inline-flex;
      align-items: center;
      gap: 0.4rem;
      font-size: 0.8rem;
      padding: 0.35rem 0.75rem;
      border-radius: 6px;
      background: var(--bg-elevated);
      border: 1px solid var(--border-subtle);
    }

    .badge-dot {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: var(--accent-emerald);
      box-shadow: 0 0 8px var(--accent-emerald);
      animation: pulse 2s infinite;
    }

    @keyframes pulse {
      0% { opacity: 1; }
      50% { opacity: 0.4; }
      100% { opacity: 1; }
    }

    main {
      padding: 2rem;
      max-width: 1600px;
      margin: 0 auto;
      width: 100%;
      display: flex;
      flex-direction: column;
      gap: 1.5rem;
      flex: 1;
    }

    /* PIPELINE STEPPER */
    .stepper-card {
      background: var(--bg-surface);
      backdrop-filter: blur(16px);
      border: 1px solid var(--border-subtle);
      border-radius: 12px;
      padding: 1.25rem 1.75rem;
      box-shadow: 0 8px 32px rgba(0,0,0,0.3);
    }

    .stepper-title {
      font-size: 0.85rem;
      color: var(--text-muted);
      text-transform: uppercase;
      letter-spacing: 0.05em;
      margin-bottom: 1rem;
      font-weight: 600;
    }

    .stepper-track {
      display: grid;
      grid-template-columns: repeat(5, 1fr);
      gap: 1rem;
      position: relative;
    }

    .step-item {
      display: flex;
      flex-direction: column;
      gap: 0.5rem;
    }

    .step-bar {
      height: 6px;
      background: rgba(255, 255, 255, 0.1);
      border-radius: 9999px;
      overflow: hidden;
      position: relative;
    }

    .step-progress {
      height: 100%;
      width: 0%;
      background: linear-gradient(90deg, var(--accent-cyan), var(--accent-emerald));
      border-radius: 9999px;
      transition: width 0.4s ease;
    }

    .step-label {
      font-size: 0.85rem;
      font-weight: 500;
      color: var(--text-muted);
    }

    .step-item.active .step-label {
      color: var(--accent-cyan);
      font-weight: 600;
    }

    .step-item.completed .step-label {
      color: var(--accent-emerald);
    }

    .session-card {
      display: grid;
      grid-template-columns: 1.2fr repeat(4, 1fr);
      gap: .75rem;
      padding: 1rem 1.25rem;
      background: var(--bg-surface);
      border: 1px solid var(--border-subtle);
      border-radius: 12px;
    }
    .metric-label { color: var(--text-muted); font-size: .7rem; text-transform: uppercase; letter-spacing: .06em; }
    .metric-value { margin-top: .25rem; font-variant-numeric: tabular-nums; font-family: var(--font-mono); font-size: .9rem; }

    /* GRID DE CONTENIDO */
    .dashboard-grid {
      display: grid;
      grid-template-columns: 1.1fr 1fr;
      gap: 1.5rem;
      flex: 1;
    }

    .panel-card {
      background: var(--bg-surface);
      backdrop-filter: blur(16px);
      border: 1px solid var(--border-subtle);
      border-radius: 12px;
      display: flex;
      flex-direction: column;
      overflow: hidden;
      box-shadow: 0 8px 32px rgba(0,0,0,0.3);
    }

    .panel-header {
      padding: 1rem 1.5rem;
      border-bottom: 1px solid var(--border-subtle);
      display: flex;
      align-items: center;
      justify-content: space-between;
      background: rgba(0, 0, 0, 0.15);
    }

    .panel-title {
      font-size: 1rem;
      font-weight: 600;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .search-input {
      background: rgba(0, 0, 0, 0.3);
      border: 1px solid var(--border-subtle);
      color: var(--text-main);
      padding: 0.35rem 0.75rem;
      border-radius: 6px;
      font-size: 0.85rem;
      font-family: var(--font-ui);
      outline: 2px solid transparent;
      outline-offset: 2px;
    }

    .search-input:focus {
      border-color: var(--accent-cyan);
    }

    .search-input:focus-visible { outline-color: var(--accent-cyan); }

    .panel-body {
      padding: 1.25rem 1.5rem;
      flex: 1;
      overflow-y: auto;
      max-height: 550px;
    }

    /* TABLA DE APIS */
    .api-group {
      margin-bottom: 1.25rem;
      border: 1px solid var(--border-subtle);
      border-radius: 8px;
      overflow: hidden;
    }

    .api-header {
      background: rgba(255, 255, 255, 0.03);
      padding: 0.75rem 1rem;
      display: flex;
      align-items: center;
      justify-content: space-between;
      cursor: pointer;
    }

    .dll-name {
      font-family: var(--font-mono);
      font-size: 0.9rem;
      color: var(--accent-cyan);
      font-weight: 600;
    }

    .function-badge {
      font-size: 0.75rem;
      background: rgba(255, 255, 255, 0.08);
      padding: 0.15rem 0.5rem;
      border-radius: 4px;
      color: var(--text-muted);
    }

    .function-list {
      padding: 0.5rem 1rem;
      display: flex;
      flex-direction: column;
      gap: 0.35rem;
      background: rgba(0, 0, 0, 0.2);
    }

    .func-item {
      font-family: var(--font-mono);
      font-size: 0.8rem;
      color: var(--text-main);
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .func-item::before {
      content: "↳";
      color: var(--text-muted);
    }

    /* CONSOLA DE LOGS */
    .console-terminal {
      background: #060911;
      font-family: var(--font-mono);
      font-size: 0.8rem;
      padding: 1rem;
      border-radius: 8px;
      overflow-y: auto;
      height: 100%;
      min-height: 480px;
      display: flex;
      flex-direction: column;
      gap: 0.35rem;
      border: 1px solid rgba(255, 255, 255, 0.05);
    }

    .log-line {
      display: flex;
      align-items: flex-start;
      gap: 0.6rem;
      line-height: 1.4;
    }

    .log-time { color: #6b7280; font-size: 0.75rem; }
    .log-tag {
      font-weight: 600;
      padding: 0.1rem 0.35rem;
      border-radius: 3px;
      font-size: 0.7rem;
    }
    .log-tag.INFO  { background: rgba(34, 211, 238, 0.15); color: #22d3ee; }
    .log-tag.REA   { background: rgba(168, 85, 247, 0.15); color: #c084fc; }
    .log-tag.WARN  { background: rgba(245, 158, 11, 0.15); color: #fbbf24; }
    .log-tag.ERROR { background: rgba(239, 68, 68, 0.15);  color: #f87171; }
    .log-tag.BUILD { background: rgba(16, 185, 129, 0.15); color: #34d399; }
    .log-msg  { color: #e5e7eb; word-break: break-word; }

    footer {
      padding: 1rem 2rem;
      text-align: center;
      color: var(--text-muted);
      font-size: 0.8rem;
      border-top: 1px solid var(--border-subtle);
    }

    @media (max-width: 900px) {
      header { padding: 1rem; align-items: flex-start; gap: 1rem; flex-direction: column; }
      .target-badge-group { flex-wrap: wrap; gap: .5rem; }
      main { padding: 1rem; }
      .dashboard-grid { grid-template-columns: 1fr; }
      .session-card { grid-template-columns: repeat(2, 1fr); }
      .panel-body { max-height: 420px; }
    }

    @media (prefers-reduced-motion: reduce) {
      *, *::before, *::after { animation-duration: .01ms !important; transition-duration: .01ms !important; }
    }
  </style>
</head>
<body>

  <header>
    <div class="brand">
      <div class="brand-icon">⚗️</div>
      <div>
        <div style="display: flex; align-items: center; gap: 0.5rem;">
          <h1 class="brand-title">Bismuth</h1>
          <span class="brand-tag">Live Engine</span>
        </div>
      </div>
    </div>

    <div class="target-badge-group">
      <div class="badge">
        <div class="badge-dot"></div>
        <span id="targetName">Esperando objetivo...</span>
      </div>
      <div class="badge">
        <span id="targetArch" style="color: var(--accent-amber);">x64</span>
      </div>
      <div class="badge">
        <span id="targetTech" style="color: var(--accent-cyan);">Triage</span>
      </div>
    </div>
  </header>

  <main>
    <section class="session-card" aria-label="Métricas de sesión y host">
      <div><div class="metric-label">Sesión</div><div class="metric-value" id="sessionId">Preparando…</div></div>
      <div><div class="metric-label">Inicio</div><div class="metric-value" id="sessionStart">—</div></div>
      <div><div class="metric-label">Fin</div><div class="metric-value" id="sessionEnd">En curso</div></div>
      <div><div class="metric-label">Duración</div><div class="metric-value" id="sessionDuration">00:00:00</div></div>
      <div><div class="metric-label">Host</div><div class="metric-value" id="hostUsage">CPU 0% · RAM 0%</div></div>
    </section>

    <!-- STEPPER TIMELINE -->
    <section class="stepper-card">
      <div class="stepper-title">Progreso del Pipeline de Reconstrucción</div>
      <div class="stepper-track">
        <div class="step-item" id="step1">
          <div class="step-label">1. Triage PE</div>
          <div class="step-bar"><div class="step-progress" id="prog1"></div></div>
        </div>
        <div class="step-item" id="step2">
          <div class="step-label">2. Evidencias REA</div>
          <div class="step-bar"><div class="step-progress" id="prog2"></div></div>
        </div>
        <div class="step-item" id="step3">
          <div class="step-label">3. Síntesis Rust</div>
          <div class="step-bar"><div class="step-progress" id="prog3"></div></div>
        </div>
        <div class="step-item" id="step4">
          <div class="step-label">4. Cargo Build</div>
          <div class="step-bar"><div class="step-progress" id="prog4"></div></div>
        </div>
        <div class="step-item" id="step5">
          <div class="step-label">5. Validador</div>
          <div class="step-bar"><div class="step-progress" id="prog5"></div></div>
        </div>
      </div>
    </section>

    <!-- MAIN GRID -->
    <div class="dashboard-grid">
      <!-- INSPECTOR PANEL -->
      <section class="panel-card">
        <div class="panel-header">
          <div class="panel-title">
            <span>🔍 Inspector de Dependencias & APIs Win32</span>
          </div>
          <span id="apiCount" class="function-badge" aria-live="polite">0 DLLs</span>
          <label for="apiSearch" class="sr-only">Filtrar APIs o DLLs</label>
          <input type="search" id="apiSearch" name="apiSearch" class="search-input" autocomplete="off" placeholder="Filtrar APIs o DLLs…" aria-describedby="apiCount">
        </div>
        <div class="panel-body" id="apisContainer">
          <div style="color: var(--text-muted); text-align: center; padding: 2rem;">
            Cargando importaciones y recursos identificados...
          </div>
        </div>
      </section>

      <!-- LIVE CONSOLE -->
      <section class="panel-card">
        <div class="panel-header">
          <div class="panel-title">
            <span>📜 Consola & Stream de Eventos</span>
          </div>
          <span style="font-size: 0.75rem; color: var(--accent-emerald);">● Conectado en vivo</span>
        </div>
        <div class="panel-body" style="padding: 0.75rem;">
          <div class="console-terminal" id="logConsole">
            <div class="log-line">
              <span class="log-time">00:00:00</span>
              <span class="log-tag INFO">INFO</span>
              <span class="log-msg">Servidor de monitoreo de Bismuth inicializado.</span>
            </div>
          </div>
        </div>
      </section>
    </div>
  </main>

  <footer>
    Bismuth v0.1.0 &copy; 2026 Gabriel Medranda (@Gabrieledevx) &bull; Conexión directa con REA (Reverse Engineer Anything)
  </footer>

  <script>
    let cachedApis = [];

    async function pollStatus() {
      try {
        const res = await fetch('/api/status');
        if (!res.ok) return;
        const data = await res.json();
        const state = data.state;
        const triage = data.triage;
        updateSession(state.session);

        if (state.target_file) {
          document.getElementById('targetName').innerText = state.target_file.split('\\').pop().split('/').pop();
        }
        if (state.architecture) {
          document.getElementById('targetArch').innerText = state.architecture;
        }
        if (state.technology) {
          document.getElementById('targetTech').innerText = state.technology;
        }

        // Actualizar Stepper según stage y progreso
        updateStepper(state.stage, state.progress_pct);

        // Actualizar Logs en vivo
        if (state.logs && state.logs.length > 0) {
          const consoleEl = document.getElementById('logConsole');
          consoleEl.innerHTML = state.logs.map(l => `
            <div class="log-line">
              <span class="log-time">${l.timestamp}</span>
              <span class="log-tag ${l.level}">${l.level}</span>
              <span class="log-msg">${escapeHtml(l.message)}</span>
            </div>
          `).join('');
          consoleEl.scrollTop = consoleEl.scrollHeight;
        }

        // Actualizar APIs si llegaron en el triage
        if (triage && triage.imported_apis) {
          cachedApis = triage.imported_apis;
          renderApis(document.getElementById('apiSearch').value);
        }

      } catch (err) {
        console.error("Error al consultar estado:", err);
      }
    }

    function updateSession(session) {
      if (!session) return;
      document.getElementById('sessionId').textContent = session.id || '—';
      document.getElementById('sessionStart').textContent = session.started_at || '—';
      document.getElementById('sessionEnd').textContent = session.finished_at || 'En curso';
      document.getElementById('sessionDuration').textContent = formatDuration(session.elapsed_ms || 0);
      const host = session.host || {};
      const total = host.memory_total_bytes || 0;
      const used = host.memory_used_bytes || 0;
      const memoryPct = total ? Math.round((used / total) * 100) : 0;
      document.getElementById('hostUsage').textContent = `CPU ${Math.round(host.cpu_percent || 0)}% · RAM ${memoryPct}% (${formatBytes(used)}/${formatBytes(total)})`;
    }

    function formatDuration(ms) {
      const seconds = Math.floor(ms / 1000);
      return `${String(Math.floor(seconds / 3600)).padStart(2, '0')}:${String(Math.floor(seconds / 60) % 60).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`;
    }

    function formatBytes(bytes) {
      if (!bytes) return '0 B';
      const units = ['B', 'KB', 'MB', 'GB', 'TB'];
      const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
      return `${(bytes / (1024 ** index)).toFixed(index ? 1 : 0)} ${units[index]}`;
    }

    function updateStepper(stage, pct) {
      const p1 = document.getElementById('prog1');
      const p2 = document.getElementById('prog2');
      const p3 = document.getElementById('prog3');
      const p4 = document.getElementById('prog4');
      const p5 = document.getElementById('prog5');

      if (pct >= 20) p1.style.width = '100%';
      if (pct >= 40) p2.style.width = '100%';
      if (pct >= 65) p3.style.width = '100%';
      if (pct >= 90) p4.style.width = '100%';
      if (pct >= 100) p5.style.width = '100%';
    }

    function renderApis(filter) {
      const container = document.getElementById('apisContainer');
      const q = filter.toLowerCase().trim();

      const filtered = cachedApis.filter(g => 
        g.dll.toLowerCase().includes(q) || g.functions.some(f => f.toLowerCase().includes(q))
      );

      document.getElementById('apiCount').textContent = `${filtered.length} DLL${filtered.length === 1 ? '' : 's'} mostradas`;

      if (filtered.length === 0) {
        container.innerHTML = '<div style="color: var(--text-muted); text-align: center; padding: 2rem;">No se encontraron APIs con ese criterio.</div>';
        return;
      }

      container.innerHTML = filtered.map(group => `
        <div class="api-group">
          <div class="api-header">
            <span class="dll-name">${escapeHtml(group.dll)}</span>
            <span class="function-badge">${group.functions.length} funciones</span>
          </div>
          <div class="function-list">
            ${group.functions.slice(0, 10).map(f => `<div class="func-item">${escapeHtml(f)}</div>`).join('')}
            ${group.functions.length > 10 ? `<div style="font-size: 0.75rem; color: #9ca3af; padding-top: 0.2rem;">... y ${group.functions.length - 10} funciones más</div>` : ''}
          </div>
        </div>
      `).join('');
    }

    document.getElementById('apiSearch').addEventListener('input', (e) => {
      renderApis(e.target.value);
    });

    function escapeHtml(text) {
      const div = document.createElement('div');
      div.innerText = text;
      return div.innerHTML;
    }

    // Polling cada segundo
    setInterval(pollStatus, 1500);
    pollStatus();
  </script>
</body>
</html>
"#
}
