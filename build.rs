use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=src/services/sparks/mod.rs");

    let mod_path = "src/services/sparks/mod.rs";

    let mod_content = fs::read_to_string(mod_path).unwrap_or_else(|_| {
        panic!("Failed to read {}", mod_path);
    });

    let mut sparks = Vec::new();
    for line in mod_content.lines() {
        if line.trim().starts_with("pub mod ") && !line.contains("registry") {
            let module_name = line.trim().strip_prefix("pub mod ").unwrap().trim_end_matches(';').trim();

            if !module_name.is_empty() {
                sparks.push(module_name);
            }
        }
    }

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("spark_registry.rs");

    let mut output = String::new();

    output.push_str("// Auto-generated list of available sparks\n");
    output.push_str("pub const AVAILABLE_SPARKS: &[&str] = &[\n");
    for spark in &sparks {
        output.push_str(&format!("    \"{}\",\n", spark));
    }
    output.push_str("];\n\n");

    output.push_str("// Auto-generated registration code\n");
    output.push_str("pub fn register_all_discovered_sparks() {\n");
    for spark in &sparks {
        output.push_str(&format!("    register_spark(\"{0}\", {0}::create_spark);\n", spark));
    }
    output.push_str("}\n");

    let mut file = fs::File::create(&dest_path).unwrap();
    file.write_all(output.as_bytes()).unwrap();
}
