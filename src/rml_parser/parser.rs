
use crate::ResultApp;
use crate::errors::ApplicationErrors;
use crate::{info, warning, error};
use super::tokenize::Token;

use crate::mappings::parts::Parts;
use crate::mappings::maps::Mapping;
use crate::mappings::AcceptedType;

use std::collections::HashMap;

use colored::*;

use regex::Regex;


const ERROR_PROMPT_LINES: i32 = 3;
fn __find_start_line_from_point(stream_tokens: &Vec<Token>, start_idx: usize, nline: i32, dir: bool) -> usize {
    let total = stream_tokens.len();

    let mut read_idx: usize  = start_idx;
    let mut found_lines = -1;

    while let Some(token) = stream_tokens.get(read_idx) {

        if matches!(token, &Token::NewLine) {
            found_lines += 1;
            if found_lines == nline { break }
        }

        if read_idx == 0 { break }
        if dir { read_idx -= 1; } else { read_idx += 1; }
    }

//    if dir { read_idx + 2 } else { read_idx - 2 }
    read_idx
}

fn __string_from_tokens(stream_tokens: &Vec<Token>, start: usize, end: usize) -> ColoredString {
    let mut ret = String::new();

    let mut tokens = stream_tokens[start..end].iter(); 
    while let Some(token) = tokens.next() {
        ret.push_str(&token.to_string());
        match token { 
            Token::Quote => {
                if let Some(next_token) = tokens.next() {
                    if matches!(next_token, Token::Literal(_)) {
                        ret.push(' ');
                    }
                    ret.push_str(&next_token.to_string());
                }
            }
            Token::ArrowLeft => {
                if let Some(next_token) = tokens.next() {
                    if matches!(next_token, Token::Literal(_)) {
                        ret.push(' ');
                    }
                    ret.push_str(&next_token.to_string());
                }
            }
            Token::Literal(_) => {
                if let Some(next_token) = tokens.next() {
                    if matches!(next_token, Token::Literal(_)) {
                        ret.push(' ');
                    }
                    ret.push_str(&next_token.to_string());
                }

            }
            _ => {}
        }
    }
    ret.white()
}

fn __print_error_lines(stream_tokens: &Vec<Token>, idx: usize, error_msg: String, line: i32) {

    let start_previous_line = __find_start_line_from_point(stream_tokens, idx, ERROR_PROMPT_LINES, true);
    let start_current_line = __find_start_line_from_point(stream_tokens, idx, 0, true);

    let end_previous_line = __find_start_line_from_point(stream_tokens, idx, ERROR_PROMPT_LINES, false);
    let end_current_line = __find_start_line_from_point(stream_tokens, idx, 0, false);

    let previous_lines = __string_from_tokens(stream_tokens, start_previous_line, start_current_line);
    let current_line   = __string_from_tokens(stream_tokens, start_current_line , end_current_line  );
    let next_lines     = __string_from_tokens(stream_tokens, end_current_line   , end_previous_line );

    error!("Error In Line {}: Unavailable to Parse the Mapping", line);
    eprint!("{}", previous_lines);
    eprintln!("{}", current_line);
    
    
    let space_left : usize   = stream_tokens[start_current_line..(idx)].iter().map(|t| t.len()).sum(); 
    let space_right: usize   = stream_tokens[(idx+1)..end_current_line].iter().map(|t| t.len()).sum(); 
    let space_token: usize   = stream_tokens[idx].len();

    eprintln!("{}{}{}", " ".repeat(space_left), "^".repeat(space_token).red(), " ".repeat(space_right));
    eprintln!("{}{}"  , " ".repeat(space_left - 1), error_msg.red());
    
    eprintln!("{}", next_lines);

}

fn parse_tokens(stream_tokens: Vec<Token>) -> ResultApp<Vec<Mapping>> {
    let mut prefix_dict: HashMap<String, String> = HashMap::new();
    
    let mut rml: Option<String> = None;
    let mut rr : Option<String> = None;

    let total_tokens = stream_tokens.len();

    let mut idx = 0;
    let mut last_line = -1;
    let mut last_line_pos = 0;
    while idx < stream_tokens.len() {
        let token = &stream_tokens[idx];
        match token {
            Token::AtSign => {
                if (idx + 8) < total_tokens {
                    
                    return Err(ApplicationErrors::UnknownToken(last_line))                    
                }

                // let used_tokens = &stream_tokens[idx..(idx + 8)];
                // let operator = match used_

            },
            Token::ArrowLeft => todo!(),
            Token::Literal(_) => todo!(),

            Token::Comma => todo!(),
            Token::Dot => todo!(),
            Token::DotComma => todo!(),

            Token::BracketLeft => todo!(),

            Token::NewLine => {
                last_line += 1;
                last_line_pos = idx;
            },

            Token::Quote => todo!(),
            Token::DoubleQuote => todo!(),

            _ => { return Err(ApplicationErrors::UnknownToken(last_line + 1)) }
        }

        idx += 1
    }


    Ok(Vec::new())   
}


