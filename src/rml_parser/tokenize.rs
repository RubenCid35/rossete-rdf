
pub enum Token {
    Literal(Vec<u8>), // "dsadsadsadsa"
    Comma,            // ,
    Dot,              // .
    DotComma,         // ;
    DotDot,           // :
    ArrowLeft,        // <
    ArrowRight,       // >
    BracketLeft,      // [
    BracketRight,     // ]
    NewLine,          // \n
    Hashtag,          // #
    Quote,            // `
    DoubleQuote,      // "
    AtSign,           // @
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Literal(chain) => write!(f, " Literal({}) ", String::from_utf8_lossy(&chain[..])),
            Token::Comma          => write!(f, " Comma(, )"       ),
            Token::Dot            => write!(f, " Dot(. )"         ),
            Token::DotComma       => write!(f, " DotComma(; )"    ),
            Token::DotDot         => write!(f, " DotDot(: )"      ),
            Token::ArrowLeft      => write!(f, " ArrowLeft(< )"   ),
            Token::ArrowRight     => write!(f, " ArrowRight(> )"  ),
            Token::BracketLeft    => write!(f, " BracketLeft([ )" ),
            Token::BracketRight   => write!(f, " BracketRight(] )"),
            Token::NewLine        => write!(f, " NewLine(\\n)"    ),
            Token::Hashtag        => write!(f, " Hashtag(#)"      ),
            Token::Quote          => write!(f, " Quote(')"        ),
            Token::DoubleQuote    => write!(f, " DoubleQuote(\")" ),
            Token::AtSign         => write!(f, " AtSign(@)"       ),
        }
    }
}

pub fn tokenize_file(file: String) -> Vec<Token> {
    let mut tokens = Vec::with_capacity(128);

    let mut file_bytes = file.bytes();
    
    let mut force_literal = false;
    while let Some(b) = file_bytes.next() {
        match b {
            b' '  => continue,
            b'\t' => continue,

            b'.'  if !force_literal => tokens.push(Token::Dot),
            b','  => tokens.push(Token::Comma),
            b':'  => tokens.push(Token::DotDot),
            b';'  => tokens.push(Token::DotComma),
            b'<'  => tokens.push(Token::ArrowLeft),
            b'>'  => tokens.push(Token::ArrowRight),
            b'['  => tokens.push(Token::BracketLeft),
            b']'  => tokens.push(Token::BracketRight),
            b'\n' => tokens.push(Token::NewLine), 
            b'\r' => tokens.push(Token::NewLine),
            b'#'  => tokens.push(Token::Hashtag),
            b'\'' => {
                force_literal = true;
                tokens.push(Token::Quote);
            }
            b'"'  => {
                force_literal = true;
                tokens.push(Token::DoubleQuote);
            }
            b'@'  => tokens.push(Token::AtSign),
            _ => { // Literal
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
                while let Some(ob) = file_bytes.next() {
                    match ob {
                        // literal case
                        b'\'' if is_literal => {
                            end_token = Some(Token::Quote);
                            break;
                        }
                        b'"' if is_literal => {
                            end_token = Some(Token::DoubleQuote);
                            break;
                        }
                        // uri case
                        b'>' if is_uri => {
                            end_token = Some(Token::ArrowRight);
                            break;
                        }

                        // predicate case
                        b' ' if !is_literal && !is_uri => break,
                        b';' if !is_literal && !is_uri => {
                            end_token = Some(Token::Comma);
                            break;
                        }
                        b'.' if !is_literal && !is_uri => {
                            end_token = Some(Token::Dot);
                            break;
                        }
                        b':' if !is_literal && !is_uri => {
                            end_token = Some(Token::DotDot);
                            break;
                        }
                        
                        b'\n' if !is_literal && !is_uri => {
                            end_token = Some(Token::NewLine);
                            break;
                        }
                        _  => {}
                    }

                    buffer.push(ob);
                }

                force_literal = false;
                tokens.push(Token::Literal(buffer));
                if let Some(token_end) = end_token {
                    tokens.push(token_end);
                }
            }
        }
    };

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
    ".to_string();

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
    "#.to_string();

    let tokens = tokenize_file(text);
    for token in tokens.iter() {
        eprint!("{:#?}", token);
    };
    eprintln!("");
}



