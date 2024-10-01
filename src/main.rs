use rossete_rdf::rml_parser::lex::Lexer;
use std::env::args;

fn get_text() -> String {
    let mut arguments = args();
    _ = arguments.next();

    arguments
        .next()
        .unwrap_or("@prefix rr: <www.example.com>.".to_string())
}

fn main() -> miette::Result<()> {
    let text = get_text();
    println!("original text: {text}");
    let lexer = Lexer::new(&text);
    for token in lexer {
        match token {
            Ok(token) => println!("{token}"),
            Err(error) => { Err(error)? }
        }
    }
    Ok(())
}
