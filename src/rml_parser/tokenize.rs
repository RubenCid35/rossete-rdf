#[derive(Clone)]
pub enum Token {
    Literal(String), // "dsadsadsadsa"
    Comma,           // ,
    Dot,             // .
    DotComma,        // ;
    DotDot,          // :
    ArrowLeft,       // <
    ArrowRight,      // >
    BracketLeft,     // [
    BracketRight,    // ]
    NewLine,         // \n
    Hashtag,         // #
    Quote,           // `
    DoubleQuote,     // "
    AtSign,          // @
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Literal(chain) => write!(f, " Literal({}) ", chain),
            Token::Comma => write!(f, " Comma(, )"),
            Token::Dot => write!(f, " Dot(. )"),
            Token::DotComma => write!(f, " DotComma(; )"),
            Token::DotDot => write!(f, " DotDot(: )"),
            Token::ArrowLeft => write!(f, " ArrowLeft(< )"),
            Token::ArrowRight => write!(f, " ArrowRight(> )"),
            Token::BracketLeft => write!(f, " BracketLeft([ )"),
            Token::BracketRight => write!(f, " BracketRight(] )"),
            Token::NewLine => write!(f, " NewLine(\\n)"),
            Token::Hashtag => write!(f, " Hashtag(#)"),
            Token::Quote => write!(f, " Quote(')"),
            Token::DoubleQuote => write!(f, " DoubleQuote(\")"),
            Token::AtSign => write!(f, " AtSign(@)"),
        }
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Literal(chain) => write!(f, "{}", chain),
            Token::Comma => write!(f, ","),
            Token::Dot => write!(f, "."),
            Token::DotComma => write!(f, ";"),
            Token::DotDot => write!(f, ":"),
            Token::ArrowLeft => write!(f, "<"),
            Token::ArrowRight => write!(f, ">"),
            Token::BracketLeft => write!(f, "["),
            Token::BracketRight => write!(f, "]"),
            Token::NewLine => write!(f, "\n"),
            Token::Hashtag => write!(f, "#"),
            Token::Quote => write!(f, "'"),
            Token::DoubleQuote => write!(f, "\""),
            Token::AtSign => write!(f, "@"),
        }
    }
}

impl Token {
    pub fn len(&self) -> usize {
        match self {
            Self::Literal(text) => text.len(),
            Self::ArrowLeft => 2,
            _ => 1,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Token::Literal(chain) => chain.clone(),
            Token::Comma => ",".to_string(),
            Token::Dot => ".".to_string(),
            Token::DotComma => ";".to_string(),
            Token::DotDot => ":".to_string(),
            Token::ArrowLeft => "<".to_string(),
            Token::ArrowRight => ">".to_string(),
            Token::BracketLeft => "[".to_string(),
            Token::BracketRight => "]".to_string(),
            Token::NewLine => "\n".to_string(),
            Token::Hashtag => "#".to_string(),
            Token::Quote => "'".to_string(),
            Token::DoubleQuote => "\"".to_string(),
            Token::AtSign => "@".to_string(),
        }
    }
}

pub fn tokenize_file(file: String) -> Vec<Token> {
    let mut tokens = Vec::with_capacity(128);

    let mut file_chars = file.chars();

    let mut force_literal = false;
    while let Some(b) = file_chars.next() {
        match b {
            ' ' => continue,
            '\t' => continue,

            '.' if !force_literal => tokens.push(Token::Dot),
            ',' => tokens.push(Token::Comma),
            ':' => tokens.push(Token::DotDot),
            ';' => tokens.push(Token::DotComma),
            '<' => tokens.push(Token::ArrowLeft),
            '>' => tokens.push(Token::ArrowRight),
            '[' => tokens.push(Token::BracketLeft),
            ']' => tokens.push(Token::BracketRight),
            '\n' => tokens.push(Token::NewLine),
            '\r' => tokens.push(Token::NewLine),
            '#' => tokens.push(Token::Hashtag),
            '\'' => {
                force_literal = true;
                tokens.push(Token::Quote);
            }
            '"' => {
                force_literal = true;
                tokens.push(Token::DoubleQuote);
            }
            '@' => tokens.push(Token::AtSign),
            _ => {
                // Literal
                let last_token = tokens
                    .last()
                    .expect("At least there should have been the symbol @ of the prefix section");
                let (is_uri, is_literal) = match last_token {
                    Token::ArrowLeft | Token::Hashtag => (true, false),
                    Token::Quote | Token::DoubleQuote => (false, true),
                    _ => (false, false),
                };
                let mut buffer = vec![b];
                let mut end_token = None;
                while let Some(ob) = file_chars.next() {
                    match ob {
                        // literal case
                        '\'' if is_literal => {
                            end_token = Some(Token::Quote);
                            break;
                        }
                        '"' if is_literal => {
                            end_token = Some(Token::DoubleQuote);
                            break;
                        }
                        // uri case
                        '>' if is_uri => {
                            end_token = Some(Token::ArrowRight);
                            break;
                        }

                        // predicate case
                        ' ' if !is_literal && !is_uri => break,
                        ';' if !is_literal && !is_uri => {
                            end_token = Some(Token::Comma);
                            break;
                        }
                        '.' if !is_literal && !is_uri => {
                            end_token = Some(Token::Dot);
                            break;
                        }
                        ':' if !is_literal && !is_uri => {
                            end_token = Some(Token::DotDot);
                            break;
                        }

                        '\n' if !is_literal && !is_uri => {
                            end_token = Some(Token::NewLine);
                            break;
                        }
                        _ => {}
                    }

                    buffer.push(ob);
                }

                force_literal = false;
                tokens.push(Token::Literal(buffer.iter().collect::<String>()));
                if let Some(token_end) = end_token {
                    tokens.push(token_end);
                }
            }
        }
    }

    tokens
}

#[cfg(test)]
#[test]
fn test_tokenize_small() {
    let text = "
    @prefix rr: <http://www.w3.org/ns/r2rml#>.
    @prefix rml: <http://semweb.mmlab.be/ns/rml#>.
    @prefix ql: <http://semweb.mmlab.be/ns/ql#>.
    @prefix transit: <http://vocab.org/transit/terms/>.
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#>.
    @prefix wgs84_pos: <http://www.w3.org/2003/01/geo/wgs84_pos#>.
    @base <http://example.com/ns#>.
    "
    .to_string();

    let tokens = tokenize_file(text);
    for token in tokens.iter() {
        eprint!("{:#?}", token);
    }
    eprintln!("");
}

#[cfg(test)]
#[test]
fn test_tokenize_with_map() {
    let text = r#"@prefix rr: <http://www.w3.org/ns/r2rml#>.
    @prefix rml: <http://semweb.mmlab.be/ns/rml#>.
    @prefix ql: <http://semweb.mmlab.be/ns/ql#>.
    @prefix transit: <http://vocab.org/transit/terms/>.
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#>.
    @prefix wgs84_pos: <http://www.w3.org/2003/01/geo/wgs84_pos#>.
    @base <http://example.com/ns#>.
    
    <#AirportMapping> a rr:TriplesMap;
      rml:logicalSource [
        rml:source "./examples/data/file-1.csv" ;
        rml:referenceFormulation ql:CSV
      ];
      rr:subjectMap [
        rr:template "http://airport.example.com/{id}";
        rr:class transit:Stop
      ];
    
      rr:predicateObjectMap [
        rr:predicate transit:route;
        rr:objectMap [
          rml:reference "stop";
          rr:datatype xsd:int
          ]
        ];
    
      rr:predicateObjectMap [
        rr:predicate wgs84_pos:lat;
        rr:objectMap [
          rml:reference "latitude"
        ]
      ];
    
      rr:predicateObjectMap [
        rr:predicate wgs84_pos:long;
        rr:objectMap [
          rml:reference "longitude"
        ]
      ].
    "#
    .to_string();

    let tokens = tokenize_file(text);
    for token in tokens.iter() {
        eprint!("{:#?}", token);
    }
    eprintln!("");
}
