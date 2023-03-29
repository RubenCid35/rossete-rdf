
use crate::ResultApp;
use crate::errors::ApplicationErrors;
use crate::{info, warning, error};
use super::tokenize::Token;


use crate::mappings::parts::Parts;
use crate::mappings::maps::Mapping;
use crate::mappings::AcceptedType;

use std::collections::HashMap;

use regex::Regex;


const ERROR_PROMPT_LINES: i32 = 1;
fn __find_start_line_from_point(stream_tokens: &Vec<Token>, start_idx: usize, dir: bool) -> usize {
    let total = stream_tokens.len();

    let mut read_idx: usize  = 0;
    let mut found_lines = -1;

    while let Some(token) = stream_tokens.get(read_idx) {

        if matches!(token, &Token::NewLine) {
            found_lines += 1;
            if found_lines == ERROR_PROMPT_LINES {
                if dir { read_idx += 1; } else { read_idx -= 1; }
                break
            }
        }

        if dir { read_idx -= 1; } else { read_idx += 1; }
        if read_idx <      0 { break }
        if read_idx >= total { break }
    }

    read_idx
}

fn __string_from_tokens(stream_tokens: &Vec<Token>, start: usize, end: usize) -> String {
    todo!()
}

fn __print_error_lines(stream_tokens: &Vec<Token>, idx: usize, error_msg: String) {
    
    // Read Line From 

}


fn parse_tokens(stream_tokens: Vec<Token>) -> ResultApp<Vec<Mapping>> {
    let mut prefix_dict: HashMap<String, String> = HashMap::new();
    
    let total_tokens = stream_tokens.len();

    let mut idx = 0;
    let mut last_line = -1;
    let mut last_line_pos = 0;
    while idx < stream_tokens.len() {
        let token = &stream_tokens[idx];
        match token {
            Token::AtSign => {
                if (idx + 8) < total_tokens {
                    return Err(ApplicationErrors::IncorrectMappingFormat)                    
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

            _ => 
        }

        idx += 1
    }


    Ok(Vec::new())   
}