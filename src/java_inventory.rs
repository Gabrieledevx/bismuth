use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs::File;
use std::path::Path;
use zip::ZipArchive;

#[derive(Debug, Serialize)]
pub struct JavaInventory {
    pub jar: String,
    pub class_count: usize,
    pub package_count: usize,
    pub resource_count: usize,
    pub manifest_present: bool,
    pub packages: Vec<String>,
    pub classes: Vec<String>,
    pub resources: Vec<String>,
}

pub fn inspect_jar(jar_path: &Path) -> Result<JavaInventory> {
    let file =
        File::open(jar_path).with_context(|| format!("No se pudo abrir {}", jar_path.display()))?;
    let mut archive = ZipArchive::new(file).context("El JAR no es un ZIP válido")?;
    let mut packages = BTreeSet::new();
    let mut classes = Vec::new();
    let mut resources = Vec::new();
    let mut manifest_present = false;

    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let name = entry.name().replace('\\', "/");
        if name == "META-INF/MANIFEST.MF" {
            manifest_present = true;
        }
        if name.ends_with("/") {
            continue;
        }
        if let Some(class_name) = name.strip_suffix(".class") {
            let class_name = class_name.replace('/', ".");
            if let Some((package, _)) = class_name.rsplit_once('.') {
                packages.insert(package.to_string());
            }
            classes.push(class_name);
        } else {
            resources.push(name);
        }
    }

    classes.sort();
    resources.sort();
    Ok(JavaInventory {
        jar: jar_path.display().to_string(),
        class_count: classes.len(),
        package_count: packages.len(),
        resource_count: resources.len(),
        manifest_present,
        packages: packages.into_iter().collect(),
        classes,
        resources,
    })
}
