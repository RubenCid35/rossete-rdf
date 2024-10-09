use clap::Parser as ArgParser;
use rossete_rdf::rml_parser::config::ParseFileConfig;
use rossete_rdf::rml_parser::parser::Parser;
use std::io::prelude::*;
use std::path::PathBuf;

/// Simple program to greet a person
#[derive(ArgParser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    file: Option<PathBuf>,

    #[arg(short, long, default_value_t = String::from("@prefix rr: <www.sample.text>.\n<#dsadsa> a rr:logicalSource."))]
    text: String,

    #[arg(short, long, help="Hide Warnings")]
    silent: bool,
}

#[inline]
fn get_text_file(file_name: &PathBuf) -> String {
    let mut ret = Vec::with_capacity(10_000);
    let mut mapping_file = std::fs::File::open(file_name).unwrap();
    let _ = mapping_file.read_to_end(&mut ret);

    String::from_utf8_lossy(&ret).to_string()
}

impl Args {
    pub fn get_text(&self) -> String {
        if let Some(file) = &self.file {
            get_text_file(file)
        } else {
            self.text.clone()
        }
    }
    pub fn get_config(&self) -> ParseFileConfig {
        if let Some(file) = &self.file {
            ParseFileConfig {
                file_path: file.clone(),
                silent: self.silent
            }
        } else {
            ParseFileConfig {
                file_path: PathBuf::from("example.text"),
                silent: self.silent
            }
        }
    }
}

fn main() -> miette::Result<()> {
    let args = Args::parse();
    let text = args.get_text();
    let config = args.get_config();

    let mut parser = Parser::new(&config, &text);
    parser.parse_structures()?;

    Ok(())
}
