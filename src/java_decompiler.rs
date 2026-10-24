use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn decompile_selected(
    jar: &Path,
    output: &Path,
    package: Option<&str>,
    requested: &[String],
    limit: usize,
    jadx_override: Option<&Path>,
) -> Result<()> {
    if !jar.is_file() {
        bail!("El JAR no existe: {}", jar.display());
    }
    let jadx = jadx_override
        .map(PathBuf::from)
        .or_else(find_jadx)
        .context("No se encontró JADX. Usa --jadx o configura JADX_PATH.")?;
    fs::create_dir_all(output)?;

    let inventory = crate::java_inventory::inspect_jar(jar)?;
    let mut candidates: Vec<String> = if requested.is_empty() {
        inventory.classes
    } else {
        requested.to_vec()
    };
    if let Some(prefix) = package {
        candidates.retain(|class| class.starts_with(prefix));
    }
    let mut processed = 0usize;
    let mut skipped = 0usize;
    for class_name in candidates {
        if processed >= limit {
            break;
        }
        let relative = format!("{}.java", class_name.replace('.', "\\"));
        let destination = output.join(relative);
        if destination.is_file() {
            skipped += 1;
            continue;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let temp_dir = output.join(".jadx-temp").join(class_name.replace('.', "_"));
        fs::create_dir_all(&temp_dir)?;
        let mut command = Command::new(&jadx);
        if std::env::var_os("JAVA_HOME").is_none() {
            let bundled_java = Path::new(r"C:\Program Files\Microsoft\jdk-25.0.4.101-hotspot");
            if bundled_java.join(r"bin\java.exe").is_file() {
                command.env("JAVA_HOME", bundled_java);
            }
        }
        let status = command
            .args(["--single-class", &class_name, "--single-class-output"])
            .arg(&temp_dir)
            .args(["--no-res", "--threads-count", "1", "--log-level", "quiet"])
            .arg(jar)
            .status()
            .with_context(|| format!("No se pudo ejecutar JADX para {}", class_name))?;
        if status.success() || temp_dir.exists() {
            let generated = find_java_file(&temp_dir)?;
            if let Some(generated) = generated {
                fs::copy(generated, &destination)?;
                processed += 1;
                println!("[+] {}", class_name);
            } else {
                eprintln!("[!] JADX terminó sin generar Java para {}", class_name);
            }
        } else {
            eprintln!("[!] JADX no pudo descompilar {}", class_name);
        }
    }
    println!(
        "Clases nuevas: {} | Existentes omitidas: {} | Salida: {}",
        processed,
        skipped,
        output.display()
    );
    Ok(())
}

fn find_java_file(dir: &Path) -> Result<Option<PathBuf>> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            if let Some(found) = find_java_file(&path)? {
                return Ok(Some(found));
            }
        } else if path.extension().is_some_and(|ext| ext == "java") {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

fn find_jadx() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("JADX_PATH") {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let candidate = PathBuf::from(r"C:\Tools\bismuth\jadx-1.5.6-portable\bin\jadx.bat");
    candidate.is_file().then_some(candidate)
}
