use std::path::PathBuf;

use miette::Report;
use rossete_rdf::core::base_function;
use rossete_rdf::mapping::rml::parser;
use rossete_rdf::{GlobalSettings, MaterializerError};

fn main() {
    if let Err(e) = run() {
        eprintln!("{:?}", Report::new(e));
        std::process::exit(1);
    }
}

fn run() -> Result<(), MaterializerError> {
    let mut global_settings = GlobalSettings::new(PathBuf::from("mappings"));
    println!("Global settings: {:?}", global_settings);
    global_settings.disable_warnings();
    println!("Global settings: {:?}", global_settings);

    base_function();

    parser::parse_rml_demo()?;

    Ok(())
}

