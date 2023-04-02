
use crate::ResultApp;
use crate::errors::ApplicationErrors;
use super::tokenize::Token;

use crate::mappings::parts::Parts;
use crate::mappings::maps::Mapping;
use crate::mappings::AcceptedType;

use std::collections::HashMap;
use std::path::PathBuf;

use regex::Regex;
use lazy_static::lazy_static;

use crate::error;
use super::__print_error_lines;
use super::find_n_endline;

lazy_static! {
    static ref URL: Regex = Regex::new(r#"https?://(?:www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b(?:[-a-zA-Z0-9()@:%_\+.~#?&/=]*)"#).unwrap();
}

fn parse_tokens(stream_tokens: Vec<Token>) -> ResultApp<Vec<Mapping>> {

    let mut prefix_dict: HashMap<String, String> = HashMap::new();
    let mut mappings = Vec::new();

    let mut idx = 0;
    let mut last_line = 0;
    let total_tokens = stream_tokens.len();


    while idx < total_tokens {
        let token = &stream_tokens[idx];
        match token {
            Token::AtSign => {
                let (prefix, url, offset) = parse_prefix_tokens(&stream_tokens, idx, last_line)?;
                prefix_dict.insert(prefix, url);
                idx += offset;
            },
            Token::ArrowLeft => {
                let (mut mapping, lines, offset) = parse_mapping_declaration(&stream_tokens, idx, last_line + 1, &prefix_dict)?;
                let prefx = std::sync::Arc::new(prefix_dict.clone());
                mapping.change_prefixes(prefx);
                mappings.push(mapping);
                last_line = lines;
                idx += offset;
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


    Ok(mappings)   
}

fn parse_prefix_tokens(tokens: &Vec<Token>, idx: usize, line: i32) -> ResultApp<(String, String, usize)> {
    lazy_static! {
        static ref BASE: Regex = Regex::new(r#"(base|BASE)"#).unwrap();
        static ref PREFIX: Regex = Regex::new(r#"(prefix|PREFIX)"#).unwrap();
    };
    
    let mut prefix = String::new();
    let mut url = String::new();
    let mut is_base = false;

    let offset: usize;
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

fn find_abreviate(prefixes: &HashMap<String, String>, url: &str) -> ResultApp<String> {
    let ret = prefixes.iter().filter(|(_, uri)| uri.contains(url) ).map(|(pre, _)| pre).next();
    match ret {
        Some(pre) => { Ok(pre.clone()) },
        None => {
            error!("There is no prefix assigned to the url {} so there are predicates that may not be parsed correctly. \nAdd the prefix to the mapping file to continue.", url);
            return Err(ApplicationErrors::MissingPrefix)
        }
    }
}

fn parse_mapping_declaration(tokens: &Vec<Token>, start_idx: usize, line: i32, prefixes: &HashMap<String, String>) -> ResultApp<(Mapping, i32, usize)> {
    // Find RML NameSpace Prefix
    let rml = find_abreviate(prefixes, r#"http://semweb.mmlab.be/ns/rml#"#)?; 
    let rr  = find_abreviate(prefixes, r#"http://www.w3.org/ns/r2rml#"#   )?;
    let ql  = find_abreviate(prefixes, r#"http://semweb.mmlab.be/ns/ql#"# )?;

    let mut mapping = Mapping::new(String::new());

    let mut scope = 0;
    let mut lines = 0;

    let total_tokens = tokens.len();
    let mut idx: usize = start_idx;
    while idx < total_tokens {
        
        let token = &tokens[idx];
        let next_dotdot = matches!(tokens.get(idx + 1), Some(Token::DotDot));

        // TODO remove


        match token {
            Token::ArrowLeft => {
                if !matches!(tokens.get(idx + 1), Some(Token::Hashtag)) {
                    __print_error_lines(&tokens, idx + 1, "The mapping name must have the following format: <#MapName>".to_string(), lines + line);
                    return Err(ApplicationErrors::UnknownToken(line + lines))
                }
                let map_name = tokens.get(idx + 2);
                let map_name = match map_name {
                    Some(Token::Literal(name)) => name,
                    _ => {
                        __print_error_lines(&tokens, idx + 2, "The mapping name must have the following format: <#MapName>".to_string(), lines + line);
                        return Err(ApplicationErrors::UnknownToken(line + lines))
                    } 
                };
                match tokens.get(idx + 3) {
                    Some(Token::ArrowRight) => {  }
                    _ => {
                        __print_error_lines(&tokens, idx + 3, "The mapping name must have the following format: <#MapName>".to_string(), lines + line);
                        return Err(ApplicationErrors::UnknownToken(line + lines))
                    }
                };
                idx += 3;
                mapping.set_identifier(map_name.clone());
            }

            Token::Literal(pre) if next_dotdot => {
                let offset;
                if pre == &rml {
                    match tokens.get(idx + 2) {
                        Some(Token::Literal(predicate)) if predicate == "logicalSource" => {
                            if !matches!(tokens.get(idx + 3), Some(Token::BracketLeft)) {
                                __print_error_lines(tokens, idx + 3, "The logical source must be followed with `[`".to_string(), line + lines);
                                return Err(ApplicationErrors::UnknownToken(line + lines))
            
                            }
                            let (logical_source, nlines, off) = parse_logical_source(&tokens, idx + 3, lines, &rml, &ql, prefixes)?; 
                            offset = off + 3;
                            lines += nlines; 
                            mapping.add_component(logical_source);
                        }
                        _ => {
                            __print_error_lines(tokens, idx + 2, "This predicate is unknown. Can't be parsed".to_string(), line + lines);
                                    return Err(ApplicationErrors::UnknownToken(line + lines))
                        }
                    }
                }else if pre == &rr {
                    offset = 2;
                    // todo!()
                } else {
                    __print_error_lines(tokens, idx, "This predicate prefix is unknown. ".to_string(), line + lines);
                    return Err(ApplicationErrors::UnknownToken(line + lines))
                }
                idx += offset;
            }
            Token::Literal(pre)  => {
                eprintln!("Literal extra {:?}", pre);
                // todo!()
            }
            Token::DotComma => { }
            Token::Dot => { break }
            Token::NewLine => { lines += 1; }
            other=> {
                __print_error_lines(&tokens, idx, "This token cann't be parsed".to_string(), lines + line);
                return Err(ApplicationErrors::UnknownToken(line + lines))
            }
        }
        idx += 1;
    };

    // TODO Remove Arc from prefixes
    Ok(( mapping, line + lines, idx - start_idx))
}

fn parse_logical_source(stream_tokens: &Vec<Token>, start_idx: usize, line: i32, rml: &String, ql: &String, prefixes: &HashMap<String, String>) -> ResultApp<(Parts, i32, usize)> {  
    // [, rml, :, source, ", oath, ", rml, :, source, ", oath, ",rml, :, source, ", oath, ",]
    let mut idx = start_idx;
    let mut lines: i32 = line;

    let mut source = PathBuf::new();    // 0
    let mut iterator = String::new(); // 1
    let mut reference_formulation = AcceptedType::Unspecify; // 2

    let mut scope = 0;
    let mut prev_predicate = -1;
    let mut end_predicate = false;
    while idx < stream_tokens.len() {
        let token = &stream_tokens[idx];
        let next_dotdot = matches!(stream_tokens.get(idx + 1), Some(Token::DotDot));
        match token {
            Token::DotComma if end_predicate => {
                end_predicate = false;
            }
            Token::Dot => {
                if matches!(stream_tokens.get(idx + 1), Some(Token::BracketRight)) {
                    idx += 1;
                    break
                } else if matches!(stream_tokens.get(idx + 1), Some(Token::NewLine)) && matches!(stream_tokens.get(idx + 2), Some(Token::BracketRight)){
                    idx += 2;
                    break
                }
                else {
                    __print_error_lines(&stream_tokens, idx, "The dot indicates the end of a scope or the end of a list of predicates. This should be followed by ']'".to_string(), lines + line);
                    return Err(ApplicationErrors::UnknownToken(line + lines))
                }
            }
            Token::BracketLeft  => { scope += 1; }
            Token::BracketRight => { scope -= 1; }
            Token::Literal(pre) if next_dotdot => { 
                if pre == rml { 
                    match stream_tokens.get(idx + 2) {
                        Some(Token::Literal(predicate)) => {
                            match &predicate[..] {
                                "source" => {prev_predicate = 1;},
                                "iterator" => {prev_predicate = 2;},
                                "referenceFormulation" => { prev_predicate = 3; }
                                _ => {
                                    __print_error_lines(stream_tokens, idx, "This predicate is unknown. Can't be parsed".to_string(), line + lines);
                                    return Err(ApplicationErrors::UnknownToken(line))
                                }
                            }
                        }
                        _ => {
                            __print_error_lines(stream_tokens, idx, "This predicate is unknown. Can't be parsed".to_string(), lines);
                            return Err(ApplicationErrors::UnknownToken(line))
                        }
                    };
    
                    idx += 2;
                } else if (pre == ql && prev_predicate == 3) {
                    let file_type = match stream_tokens.get(idx + 2) {
                        Some(Token::Literal(ft)) => ft,
                        _ => {
                            __print_error_lines(stream_tokens, idx, "This predicate is unknown. Can't be parsed".to_string(), lines);
                            return Err(ApplicationErrors::UnknownToken(line))
                        }
                    };

                    reference_formulation = AcceptedType::from_str(&file_type);
                    idx += 2;
                    end_predicate = true;
                }
                else {
                    __print_error_lines(stream_tokens, idx, "This predicate is unknown. Can't be parsed".to_string(), lines);
                    return Err(ApplicationErrors::UnknownToken(line))
                }
            }
            Token::DoubleQuote => {
                if !matches!(stream_tokens.get(idx + 2), Some(Token::DoubleQuote)) {
                    __print_error_lines(stream_tokens, idx, "This literal must be end with a double quote \"".to_string(), lines);
                    return Err(ApplicationErrors::UnknownToken(line))
                };
                let literal = match stream_tokens.get(idx + 1) {
                    Some(Token::Literal(lit)) => lit.clone(),
                    _ => {
                        __print_error_lines(stream_tokens, idx, "This RML Map Component should not be here. This cann't be parsed".to_string(),lines);
                        return Err(ApplicationErrors::UnknownToken(line))
                    }
                };

                match prev_predicate {
                    1 => {
                        source = std::path::PathBuf::from(&literal);
                    }
                    2 => {
                        iterator = literal;
                    }
                    _ => {
                        __print_error_lines(stream_tokens, idx, "This RML Map Component should not be here. This cann't be parsed".to_string(), lines);
                        return Err(ApplicationErrors::UnknownToken(line))
                    }
                }
                end_predicate = true;
                idx += 2;
            }
            Token::NewLine => { lines += 1;}
            _ => {
                __print_error_lines(&stream_tokens, idx, "This token cann't be parsed. Logical Source".to_string(), lines);
                return Err(ApplicationErrors::UnknownToken(line))
            }
        };
        
    if scope == 0 { break }
        idx += 1;
    };

    

    let logical_source = Parts::LogicalSource { source, reference_formulation, iterator };
    Ok((logical_source, lines - line, idx - start_idx))
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
        @prefix xsd : <http://www.w3.org/2001/XMLSchema#>.
        #Esto Sobra <#hola>
        @prefix wgs84_pos: <http://www.w3.org/2003/01/geo/wgs84_pos#>.
        @base <http://example.com/ns#>. #  No dara error esto

        " .to_string();
        
        let tokens = tokenize_file(text);
        let ret = parse_tokens(tokens);
        assert!(ret.is_ok(), "The prefix parsing should work.")
    }

    #[test]
    fn test_parse_map_name () {
        let text = r#"
        @prefix rr: <http://www.w3.org/ns/r2rml#>.
        @prefix rml: <http://semweb.mmlab.be/ns/rml#>.
        @prefix ex: <http://example.com/ns#>.
        @prefix ql: <http://semweb.mmlab.be/ns/ql#>.
        @prefix transit: <http://vocab.org/transit/terms/>.
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#>.
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#>.
        @base <http://example.com/ns#>.
        
        
        # XML is not supported yet.
        <#TransportMapping>
            rml:logicalSource [
                rml:referenceFormulation ql:XPath;
                rml:source "./examples/data/file-2.xml" ;
                rml:iterator "/transport/bus".
            ].
        "#.to_string();
        let tokens = tokenize_file(text);
        let mappings = parse_tokens(tokens);
        assert!(mappings.is_ok(), "The mapping should be ok");
        let maps = mappings.unwrap();
        let map = maps.first().unwrap();
        assert_eq!(map.get_identifier(), "TransportMapping");

    }

}