
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


const ERROR_PROMPT_LINES: i32 = 2;
fn find_n_endline(stream_tokens: &Vec<Token>, start_idx: usize, nline: i32, dir: bool) -> usize {
    let total = stream_tokens.len();

    let mut read_idx: usize  = start_idx;
    let mut found_lines = -1;
    while let Some(token) = stream_tokens.get(read_idx ) {
        if matches!(token, &Token::NewLine) {
            found_lines += 1;
            if found_lines >= nline { 
                break
            }
        }
        if dir  && read_idx == 0 { return read_idx }
        if !dir && read_idx == (total - 1) { return read_idx }

        if dir { read_idx -= 1; } else { read_idx += 1; }
    
    }
    
    read_idx
    // if dir { read_idx +1 } else { read_idx - 1 }
}

fn __string_from_tokens(stream_tokens: &Vec<Token>, start: usize, end: usize, interest: bool, start_line: i32) -> ColoredString {
    let mut ret = String::with_capacity(100);
    let mut current_line = start_line;
    let mut prev_newline = ! matches!(&stream_tokens[start], &Token::NewLine);

    if start == end { return ret.white() }

    let mut tokens = stream_tokens[start..=end].iter(); 
    while let Some(token) = tokens.next() {
        if prev_newline {
            let line_number = format!("{:>3}.    ", current_line);
            ret.push_str(&line_number);
            current_line += 1;
            prev_newline = false;
        }
        ret.push_str(&token.to_string());
        match token { 
            Token::NewLine => { prev_newline = true}
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
    if interest {
        if !prev_newline { ret.push('\n'); }
        ret.white() 
    } else {
        ret.truecolor( 149, 165, 166 )
    }
}

fn __print_error_lines(stream_tokens: &Vec<Token>, idx: usize, error_msg: String, line: i32) {

    let start_previous_line = find_n_endline(stream_tokens, idx, ERROR_PROMPT_LINES, true);
    let start_current_line = find_n_endline(stream_tokens, idx, 0, true);

    let end_next_line = find_n_endline(stream_tokens, idx, ERROR_PROMPT_LINES, false);
    let end_current_line = find_n_endline(stream_tokens, idx, 0, false);


    let previous_lines = __string_from_tokens(stream_tokens, start_previous_line     , start_current_line, false, line - ERROR_PROMPT_LINES);
    let current_line   = __string_from_tokens(stream_tokens, start_current_line      , end_current_line, true, line);
    let next_lines     = __string_from_tokens(stream_tokens, end_current_line , end_next_line, false, line + 1);

    error!("Error In Line {}: Unavailable to Parse the Mapping. ", line);
    eprint!("{}", previous_lines);
    eprint!("{}", current_line);
    
    
    let space_left : usize   = stream_tokens[start_current_line..=(idx)].iter().map(|t| t.len()).sum(); 
    let space_token: usize   = stream_tokens[idx].len() + 8;
    let space_left = if space_left == 0 { 0 } else { space_left - 1 };

    eprintln!("{}{}", " ".repeat(space_left), "^".repeat(space_token + 2).red());
    eprintln!("{}{}"  , " ".repeat(space_left), error_msg.red());
    eprintln!("{}", next_lines); 

}

fn parse_tokens(stream_tokens: Vec<Token>) -> ResultApp<Vec<Mapping>> {
    let mut prefix_dict: HashMap<String, String> = HashMap::new();
    let total_tokens = stream_tokens.len();

    let mut idx = 0;
    let mut last_line = 0;

    while idx < total_tokens {
        let token = &stream_tokens[idx];
        match token {
            Token::AtSign => {
                let (prefix, url, offset) = parse_prefix_tokens(&stream_tokens, idx, last_line)?;
                prefix_dict.insert(prefix, url);
                idx += offset;
            },
            Token::ArrowLeft => {
                let (mappings, used_lines, offset) = parse_mapping_declaration(&stream_tokens, idx, last_line + 1, &prefix_dict)?;
            },
            Token::Hashtag => { // comment
                // Skip Comment
                idx = find_n_endline(&stream_tokens, idx, 0, false);
            }
            Token::NewLine => {
                last_line += 1;
            },
            _ => { return Err(ApplicationErrors::UnknownToken(last_line + 1)) }
        }

        idx += 1
    }


    Ok(Vec::new())   
}

fn parse_prefix_tokens(tokens: &Vec<Token>, idx: usize, line: i32) -> ResultApp<(String, String, usize)> {
    lazy_static! {
        static ref BASE: Regex = Regex::new(r#"(base|BASE)"#).unwrap();
        static ref PREFIX: Regex = Regex::new(r#"(prefix|PREFIX)"#).unwrap();
    };
    
    let mut prefix = String::new();
    let mut url = String::new();
    let mut is_base = false;

    let mut offset = 0;
    if let Some(Token::Literal(word)) = tokens.get(1 + idx) {
        if PREFIX.is_match(&word) {
            if let Some(Token::Literal(pref)) = tokens.get(idx + 2) {
                prefix.push_str(&pref[..]);
                if !matches!(tokens.get(idx + 3), Some(Token::DotDot)) {
                    __print_error_lines(&tokens, 2 + idx, "This token should be followed by ':'".to_string(), line);
                    return Err(ApplicationErrors::InvalidUSeOfToken)
                }
                offset = 4;
            } else {
                __print_error_lines(&tokens, 2 + idx, "This token should be the prefix asign to the url".to_string(), line);
                return Err(ApplicationErrors::InvalidUSeOfToken)
            }

        }
        else if BASE.is_match(&word) {
            offset = 2;
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
        Some(error_token) => {
            if matches!(error_token, &Token::NewLine) {
                __print_error_lines(&tokens, idx + offset + 2, "The prefix declaration must end with a dot '.'.".to_string(), line);
                return Err(ApplicationErrors::InvalidUSeOfToken)
            }
            else {
                __print_error_lines(&tokens, idx + offset + 3, "The prefix declaration must end with a dot '.'. This Token should be a dot.".to_string(), line);
                return Err(ApplicationErrors::InvalidUSeOfToken)
            }
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

    return Ok((prefix, url, offset + 3))
}

fn parse_mapping_declaration(tokens: &Vec<Token>, idx: usize, line: i32, prefixes: &HashMap<String, String>) -> ResultApp<(Mapping, i32, usize)> {

    todo!()
}

#[cfg(test)]
mod test_parsing {
    use super::*;
    use crate::rml_parser::tokenize_file;

    #[test]
    fn test_parse_prefix () {
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


        let prefix_dec = "@base  <http://www.w3.org/ns/r2rml#>.".to_string();
        let tokens = tokenize_file(prefix_dec);
        let ret = parse_prefix_tokens(&tokens, 0, 0);
        assert!(ret.is_ok(), "this should fail");
    }

    #[test]
    fn test_parse_multiple_prefix() {
        let text = "
        @prefix rr : <http://www.w3.org/ns/r2rml#>.
        @prefix rml: <http://semweb.mmlab.be/ns/rml#>.
        @prefix ql: <http://semweb.mmlab.be/ns/ql#>.
        @prefix transit : <http://vocab.org/transit/terms/>.
        @prefix xsd <http://www.w3.org/2001/XMLSchema#>.
        #Esto Sobra <#hola>
        @prefix wgs84_pos: <http://www.w3.org/2003/01/geo/wgs84_pos#>.
        @base <http://example.com/ns#>. #  No dara error esto

        " .to_string();
        
        let tokens = tokenize_file(text);
        let ret = parse_tokens(tokens);
        assert!(ret.is_err(), "The prefix parsing should work.")

    }

}