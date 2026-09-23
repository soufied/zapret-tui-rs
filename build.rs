use std::env;
use std::fs;
use std::path::Path;

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut chars = p.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect()
}

fn generate_backends() {
    let backends_dir = Path::new("src/firewalls/backends");
    let dest = backends_dir.join("_backends.rs");

    let mut backends = Vec::new();

    if let Ok(entries) = fs::read_dir(backends_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if stem != "mod" && !stem.starts_with('_') {
                        backends.push(stem.to_string());
                    }
                }
            }
        }
    }

    backends.sort_by(|a, b| {
        if a == "nftables" {
            std::cmp::Ordering::Less
        } else if b == "nftables" {
            std::cmp::Ordering::Greater
        } else {
            a.cmp(b)
        }
    });

    let mut output = String::new();

    for backend in &backends {
        output.push_str(&format!("pub mod {};\n", backend));
    }

    output.push_str("\ndefine_backends! {\n");
    for backend in &backends {
        let variant = to_pascal_case(backend);
        output.push_str(&format!(
            "    {} => {}: {}Backend as \"backend_{}\",\n",
            variant, backend, variant, backend
        ));
    }
    output.push_str("}\n");

    let unchanged = fs::read_to_string(&dest).map(|existing| existing == output).unwrap_or(false);
    if !unchanged {
        fs::write(&dest, output).unwrap();
    }

    for backend in &backends {
        println!("cargo:rerun-if-changed=src/firewalls/backends/{}.rs", backend);
    }
    println!("cargo:rerun-if-changed=src/firewalls/backends/mod.rs");
}

fn embed_windows_resources() {
    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string());

    let mut resource = winresource::WindowsResource::new();
    resource.set_manifest_file("app.manifest");
    resource.set("FileDescription", "Zapret-Rust DPI bypass controller");
    resource.set("ProductName", "Zapret-Rust");
    resource.set("LegalCopyright", "MIT");
    resource.set("OriginalFilename", "zapret-rust.exe");
    resource.set("FileVersion", &version);
    resource.set("ProductVersion", &version);

    if Path::new("assets/app.ico").exists() {
        resource.set_icon("assets/app.ico");
    }

    if let Err(error) = resource.compile() {
        println!("cargo:warning=windows resource embedding skipped: {}", error);
    }

    println!("cargo:rerun-if-changed=app.manifest");
    println!("cargo:rerun-if-changed=assets/app.ico");
}

fn main() {
    generate_backends();

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        embed_windows_resources();
    }

    println!("cargo:rerun-if-changed=build.rs");
}
