use rossete_rdf::rml_parser::lex::Lexer;
use std::{env::args, path::PathBuf};
use std::io::prelude::*;

#[allow(dead_code)]
fn get_text() -> String {
    let mut arguments = args();
    _ = arguments.next();

    arguments
        .next()
        .unwrap_or("@prefix rr: <www.example.com>.".to_string())
}

#[allow(dead_code)]
fn get_text_file(file_name: &'static str) -> String {
    let mut ret = Vec::with_capacity(10_000);
    let mut mapping_file = std::fs::File::open(PathBuf::from(file_name)).unwrap();
    let _ = mapping_file.read_to_end(&mut ret);
    
    String::from_utf8_lossy(&ret).to_string()
}

fn main() -> miette::Result<()> {
    //let text = get_text();
    let text = get_text_file("./examples/mappings/map2.ttl");
    println!("original text: {text:?}");
    let lexer = Lexer::new(&text);
    for token in lexer {
        match token {
            Ok(token) => println!("{token}"),
            Err(error) => { Err(error)? }
        }
    }
    Ok(())
}
