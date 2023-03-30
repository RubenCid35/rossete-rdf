
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
use lazy_static::lazy_static;

lazy_static! {
    static ref URL: Regex = Regex::new(r#"https?://(?:www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b(?:[-a-zA-Z0-9()@:%_\+.~#?&/=]*)"#).unwrap();
}


const ERROR_PROMPT_LINES: i32 = 3;
fn __find_start_line_from_point(stream_tokens: &Vec<Token>, start_idx: usize, nline: i32, dir: bool) -> usize {
    let total = stream_tokens.len();

    let mut read_idx: usize  = start_idx;
    let mut found_lines = 0;
    eprintln!("{}", total);
    while let Some(token) = stream_tokens.get(read_idx ) {
        if matches!(token, &Token::NewLine) {
            found_lines += 1;
            if found_lines == nline { break }
        }

        if dir  && read_idx == 0 { return read_idx }
        if !dir && read_idx >= (total - 1) { return read_idx }
    
        if dir { read_idx -= 1; } else { read_idx += 1; }
    }
    
    if dir { read_idx +1 } else { read_idx - 1 }
}

fn __string_from_tokens(stream_tokens: &Vec<Token>, start: usize, end: usize) -> ColoredString {
    let mut ret = String::new();

    if start == end { return ret.white() }

    let mut tokens = stream_tokens[start..=end].iter(); 
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


    let end_next_line = __find_start_line_from_point(stream_tokens, idx, ERROR_PROMPT_LINES, false);
    let end_current_line = __find_start_line_from_point(stream_tokens, idx, 0, false);

    eprintln!("{} {} {} {}", start_previous_line, start_current_line, end_current_line, end_next_line);

    let previous_lines = __string_from_tokens(stream_tokens, start_previous_line, start_current_line);
    let current_line   = __string_from_tokens(stream_tokens, start_current_line , end_current_line  );
    let next_lines     = __string_from_tokens(stream_tokens, end_current_line   , end_next_line );

    error!("Error In Line {}: Unavailable to Parse the Mapping. Msg: {}", line, error_msg);
    eprint!("{}", previous_lines);
    eprintln!("{}", current_line);
    
    
    let space_left : usize   = stream_tokens[start_current_line..=(idx)].iter().map(|t| t.len()).sum(); 
    let space_right: usize   = if (idx + 1) < stream_tokens.len() {
        stream_tokens[(idx+1)..=end_current_line].iter().map(|t| t.len()).sum()
    } else { 0 }; 

    let space_token: usize   = stream_tokens[idx].len();

    let space_left = if space_left == 0 { 0 } else { space_left - 1 };

    eprintln!("{}{}{}", " ".repeat(space_left), "^".repeat(space_token).red(), " ".repeat(space_right));
    eprintln!("{}{}"  , " ".repeat(space_left), error_msg.red());
    
    eprintln!("{}", next_lines);

}

fn parse_tokens(stream_tokens: Vec<Token>) -> ResultApp<Vec<Mapping>> {
    let mut prefix_dict: HashMap<String, String> = HashMap::new();
    
    let mut rml: Option<String> = None;
    let mut rr : Option<String> = None;

    let total_tokens = stream_tokens.len();

    let mut idx = 0;
    let mut last_line = 0;
    let mut last_line_pos = 0;


    while idx < total_tokens {
        let token = &stream_tokens[idx];
        match token {
            Token::AtSign => {
                if (idx + 7) < total_tokens { 
                    __print_error_lines(&stream_tokens, idx, "This possible prefix declarations has missing parts. @prefix {pre} : < url > .".to_string(), last_line);
                    return Err(ApplicationErrors::InvalidUSeOfToken)
                } 
                let (prefix, url) = parse_prefix_tokens(&stream_tokens, idx, last_line + 1)?;
                prefix_dict.insert(prefix, url);
            
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

fn parse_prefix_tokens(tokens: &Vec<Token>, idx: usize, line: i32) -> ResultApp<(String, String)> {
    lazy_static! {
        static ref BASE: Regex = Regex::new(r#"(base|BASE)"#).unwrap();
        static ref PREFIX: Regex = Regex::new(r#"(prefix|PREFIX)"#).unwrap();
    };
    
    let mut prefix = String::new();
    let mut url = String::new();

    let mut offset = 0;
    if let Some(Token::Literal(word)) = tokens.get(1 + idx) {
        if PREFIX.is_match(&word) {
            if let Some(Token::Literal(pref)) = tokens.get(idx + 2) {
                prefix.push_str(&pref[..]);
                if !matches!(tokens.get(idx + 3), Some(Token::DotDot)) {
                    __print_error_lines(&tokens, 2 + idx, "This token should be ':'".to_string(), line);
                    return Err(ApplicationErrors::InvalidUSeOfToken)
                }
                offset = 4;
            } else {
                __print_error_lines(&tokens, 2 + idx, "This token should be the prefix asign to the url".to_string(), line);
                return Err(ApplicationErrors::InvalidUSeOfToken)
            }

        }
        else if BASE.is_match(&word) {
            if !matches!(tokens.get(idx + 2), Some(Token::DotDot)) {
                __print_error_lines(&tokens, 2 + idx, "This token should be ':'".to_string(), line);
                return Err(ApplicationErrors::InvalidUSeOfToken)
            }
            prefix.push(':');
            offset = 3;
        } else {
            __print_error_lines(&tokens, 1 + idx, "Unknown Operation. This should be 'prefix' or base".to_string(), line);
            return Err(ApplicationErrors::UnknownToken(line))
            }
    }else {
        __print_error_lines(&tokens,  1 + idx, "Unknown Operation. This should be 'prefix' or base".to_string(), line);
        return Err(ApplicationErrors::InvalidUSeOfToken)
    }

    if !matches!(tokens.get(idx + offset), Some(Token::ArrowLeft)) {
        __print_error_lines(&tokens, idx + offset, "The uris must be between arrow symbols: < uri >".to_string(), line);
        return Err(ApplicationErrors::InvalidUSeOfToken)
    }

    if !matches!(tokens.get(idx + offset + 2), Some(Token::ArrowRight)) {
        __print_error_lines(&tokens, idx + offset + 2, "The uris must be between arrow symbols: < uri >".to_string(), line);
        return Err(ApplicationErrors::InvalidUSeOfToken)
    }

    match tokens.get(idx + offset + 3) {
        Some(Token::Dot) => {},
        Some(token) => {
            eprintln!("{:?}", token);
            __print_error_lines(&tokens, idx + offset + 3, "The prefix declaration must end with a dot '.'".to_string(), line);
            return Err(ApplicationErrors::InvalidUSeOfToken)
        }
        None => {
            __print_error_lines(&tokens, idx + offset + 2, "The prefix declaration must end with a dot '.'".to_string(), line);
            return Err(ApplicationErrors::InvalidUSeOfToken)
        }
    }

    if let Some(Token::Literal(pos_uri)) = tokens.get(idx + offset + 1) {
        if ! URL.is_match(&pos_uri) {
            __print_error_lines(&tokens, idx + offset + 1, "This literal should be an URI".to_string(), line);
            return Err(ApplicationErrors::InvalidUSeOfToken)        
                
        }
        url.push_str(&pos_uri);

    }else{
        __print_error_lines(&tokens, idx + offset + 1, "This literal should be an URI".to_string(), line);
        return Err(ApplicationErrors::InvalidUSeOfToken)        
    }


    return Ok((prefix, url))
}

#[cfg(test)]
mod test_parsing {
    use super::*;
    use crate::rml_parser::tokenize_file;

    #[test]
    fn test_parse_uri () {
        let prefix_dec = "@prefix rr: <http://www.w3.org/ns/r2rml#>.".to_string();
        let tokens = tokenize_file(prefix_dec);
        let ret = parse_prefix_tokens(&tokens, 0, 0);
        assert!(ret.is_ok(), "this should not fail");
        let ret = ret.unwrap();
        assert_eq!(ret.0, "rr".to_string(), "The prefix header does not match");
        assert_eq!(ret.1, "http://www.w3.org/ns/r2rml#".to_string(), "The prefix value does not match");

        let prefix_dec = "@prefix rmlk: <http://www.w3.org/ns/r2rml#>".to_string();
        let tokens = tokenize_file(prefix_dec);
        let ret = parse_prefix_tokens(&tokens, 0, 0);
        assert!(ret.is_err(), "this should fail");


        let prefix_dec = "@base: <http://www.w3.org/ns/r2rml#>.".to_string();
        let tokens = tokenize_file(prefix_dec);
        let ret = parse_prefix_tokens(&tokens, 0, 0);
        assert!(ret.is_ok(), "this should fail");
    }

}