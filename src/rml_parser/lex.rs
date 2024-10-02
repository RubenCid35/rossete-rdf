use miette::{Diagnostic, NamedSource, SourceSpan};
use std::path::PathBuf;
use thiserror::Error;

use std::iter::Iterator;

//
// -------------------------------------------------------
// -------------------------------------------------------
// Token Definition
// -------------------------------------------------------
// -------------------------------------------------------

#[derive(Debug, PartialEq)]
#[repr(u8)]
pub enum TokenKind {
    // PUNTUATION
    Dot,      // .
    Comma,    // ,
    Colon,    // :
    DotComma, // ;

    LBracket, // [
    RBracket, // ]

    // IDENT & LITERAL
    Literal, // "dsadsa>", """dsadksajdksjakdlsja"""
    URI,     // <www.example.com>
    Ident,   // <#thisMap>
    Term,    // ex:case -> term(ex), colon, term(case)

    // SPECIAL
    Prefix, // @prefix
    Base,   // @base
}

/// Token representation. It is defined by its kind and the associated literal.
/// If the kind is and URI or a string, the literal corresponds with the whole associated string.
#[derive(Debug, PartialEq)]
pub struct Token<'de> {
    /// literal representation of the token.
    literal: &'de str,
    /// Token kind.
    kind: TokenKind,
}

impl<'de> std::fmt::Display for Token<'de> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "token: {:?}\tliteral: {:?}", self.kind, self.literal)
    }
}

// -------------------------------------------------------
// -------------------------------------------------------
// Syntax Helper
// -------------------------------------------------------
// -------------------------------------------------------

/// Small help to avoid code repetition while creaing the tokens.
macro_rules! create_token {
    ($literal:expr, $kind:expr) => {
        Some(Ok(Token {
            literal: $literal,
            kind: $kind,
        }))
    };
}

macro_rules! generate_invalid_token_error {
    ($self:ident, $file:expr, $literal:expr, $loff:expr, $roff:expr) => {
        let err = InvalidTokenFound {
            src: NamedSource::new($file, $self.whole.to_string()),
            token: $literal,
            err_span: SourceSpan::from(($self.current_byte - $loff)..($self.current_byte + $roff)),
        };

        $self.found_error = true;
        return Some(Err(err.into()));
    };
}

// -------------------------------------------------------
// -------------------------------------------------------
// Lexer Errors
// -------------------------------------------------------
// -------------------------------------------------------

/// Error used to mark an unexpected end of file while lexing.
#[derive(Diagnostic, Debug, Error)]
#[error("invalid end of file in the mapping file {:?}.", .file_name)]
#[diagnostic(
    code(rml::parser::eof),
    help("This file terminated prematurely. Check if the mapping is finished.")
)]
pub struct InvalidEndOfFile {
    file_name: PathBuf,
}

/// Location and Identification of an invalid token.
#[derive(Diagnostic, Debug, Error)]
#[error("invalid token '{token}' was found.")]
#[diagnostic(
    code(rml::parser::lex::token),
    help("Invalid token was found. Consider check if the mapping is correct.")
)]
pub struct InvalidTokenFound {
    #[source_code]
    src: NamedSource<String>,

    pub token: String,

    #[label = "This token is invalid at this position."]
    err_span: SourceSpan,
}

// -------------------------------------------------------
// -------------------------------------------------------
// Lexer State
// -------------------------------------------------------
// -------------------------------------------------------

/// This lexer iterates over the characters in the text to retrieve the next token in the text
/// THe iterator returns results with the token. If there is some error, the following calls return `None`.
pub struct Lexer<'de> {
    /// Whole text. THis is used for error reporting
    whole: &'de str,
    /// remaining mapping text to be parsed
    remaining: &'de str,
    /// current position of the lexer in the whole text
    current_byte: usize,
    /// whether a error was found or not
    found_error: bool,
}

impl<'de> Lexer<'de> {
    pub fn new(src: &'de str) -> Self {
        Self {
            whole: src,
            remaining: src,
            current_byte: 0,
            found_error: false,
        }
    }
}

impl<'de> Iterator for Lexer<'de> {
    type Item = Result<Token<'de>, miette::Error>;
    fn next(&mut self) -> Option<Self::Item> {
        // if some lexical error was found, then there is no need to continue parsing the file.
        if self.found_error {
            return None;
        };

        loop {
            let mut letters = self.remaining.chars().peekable();

            // get next letter
            let c = letters.next()?;
            self.remaining = &self.remaining[c.len_utf8()..];
            self.current_byte += c.len_utf8();

            match c {
                ' ' | '\t' | '\n' | '\r' => continue,
                '[' => return create_token!("[", TokenKind::LBracket),
                ']' => return create_token!("]", TokenKind::RBracket),
                '.' => return create_token!(".", TokenKind::Dot),
                ',' => return create_token!(",", TokenKind::Comma),
                ':' => return create_token!(":", TokenKind::Colon),
                ';' => return create_token!(";", TokenKind::DotComma),
                '@' => {
                    match letters.peek() {
                        Some(l) if l.is_alphabetic() => {}
                        _ => {
                            generate_invalid_token_error!(
                                self,
                                "all test",
                                c.to_string(),
                                c.len_utf8(),
                                0
                            );
                        }
                    }
                    let mut i = 0;
                    while let Some(l) = letters.peek() {
                        if !l.is_ascii_alphanumeric() {
                            break;
                        }
                        i += l.len_utf8();
                        letters.next();
                    }

                    let literal = &self.whole[(self.current_byte)..(self.current_byte + i)];
                    if literal == "prefix" {
                        self.remaining = &self.remaining[i..];
                        self.current_byte += i;
                        return create_token!("@prefix", TokenKind::Prefix);
                    } else if literal == "base" {
                        self.remaining = &self.remaining[i..];
                        self.current_byte += i;
                        return create_token!("@base", TokenKind::Base);
                    } else {
                        generate_invalid_token_error!(
                            self,
                            "example file",
                            literal.to_string(),
                            0,
                            i
                        );
                    }
                }
                '<' => {
                    // identifiers are defined by <#identifier>.
                    // We need to find the next closing arrow to determine the end of a ident;
                    let mut i = 0;
                    let mut kind: TokenKind = TokenKind::URI;
                    if letters.peek() == Some(&'#') {
                        // This is the case if the literal corresponds with a mapping ident.
                        letters.next();
                        i += '#'.len_utf8();
                        kind = TokenKind::Ident;

                        while let Some(l) = letters.next() {
                            match l {
                                l if l.is_ascii_alphanumeric() => i += l.len_utf8(),
                                '>' => break,
                                _ => {
                                    let err = InvalidTokenFound {
                                        src: NamedSource::new(
                                            "exampledsada.rml",
                                            self.whole.to_string(),
                                        ),
                                        token: c.to_string(),
                                        err_span: SourceSpan::from(
                                            (self.current_byte + i)
                                                ..(self.current_byte + i + l.len_utf8()),
                                        ),
                                    };

                                    self.found_error = true;
                                    return Some(Err(err.into()));
                                }
                            }
                        }
                    } else {
                        // the uri and url are defined by the <uri>.
                        // It is expected that the columns are uri-encoded so there is no clossing arrow in them.
                        while let Some(l) = letters.next() {
                            match l {
                                // Allow alphanumeric characters
                                'a'..='z' | 'A'..='Z' | '0'..='9' |
                                // Allow hyphen, underscore, period, and tilde
                                '-' | '_' | '.' | '~' |
                                // Allow URL reserved characters
                                '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' |
                                ';' | '=' | ':' | '@' | '/' | '?' | '#' | '[' | ']' => {
                                    i += l.len_utf8();
                                },
                                '>' => break,
                                // Anything else is invalid
                                _ => {
                                    let err = InvalidTokenFound {
                                        src: NamedSource::new(
                                            "example.rml",
                                            self.whole.to_string(),
                                        ),
                                        token: c.to_string(),
                                        err_span: SourceSpan::from(
                                            (self.current_byte + i)..(self.current_byte + i  + l.len_utf8()),
                                        ),
                                    };
                                    self.found_error = true;
                                    return Some(Err(err.into()));
                                }
                            }
                        }
                    }

                    // remove the hashtag from the ident name
                    let hashtag_skip = if kind == TokenKind::Ident {
                        '#'.len_utf8()
                    } else {
                        0
                    };
                    let literal = &self.remaining[hashtag_skip..i];
                    self.remaining = &self.remaining[(i + '>'.len_utf8())..];
                    self.current_byte += i + '>'.len_utf8();
                    return create_token!(literal, kind);
                }
                '"' => {
                    let mut i = 0;
                    let mut scope = 0;
                    // determine the number of double-quotes in the start (the first one is ommited)
                    while let Some(l) = letters.peek() {
                        if (*l) != '"' {
                            break;
                        };
                        scope += 1;
                        letters.next();
                    }

                    let left_space = scope * '"'.len_utf8();
                    // get the inner text
                    loop {
                        let l = match letters.next() {
                            Some(l) => l,
                            None => {
                                // TODO: change error to literal not closed
                                let error = InvalidEndOfFile {
                                    file_name: PathBuf::from("example.png"),
                                };
                                return Some(Err(error.into()));
                            }
                        };
                        // if there are {scope + 1} double-quotes in a row, it is determine to be the end of the literal.s
                        if l == '"'
                            && self.remaining[(left_space + i)..(left_space + i + scope + 1)]
                                .chars()
                                .all(|c| c == '"')
                        {
                            break;
                        }
                        i += l.len_utf8();
                    }

                    let literal = &self.remaining[(left_space)..(left_space + i)];
                    self.remaining = &self.remaining[(left_space + i + scope + 1)..];
                    self.current_byte += left_space + i + scope + 1;
                    return create_token!(literal, TokenKind::Literal);
                }
                c if c.is_ascii_alphanumeric() => {
                    let mut i = 0;
                    // only ascii alphanumeric [A-Za-z0-9] are valid characters in a term ident.
                    while let Some(l) = letters.peek() {
                        if !l.is_ascii_alphanumeric() {
                            break;
                        }
                        i += l.len_utf8();
                        letters.next();
                    }

                    // given that we had consumed the first letter we required to use the original text
                    let literal =
                        &self.whole[(self.current_byte - c.len_utf8())..(self.current_byte + i)];
                    self.remaining = &self.remaining[i..];
                    self.current_byte += i;
                    return create_token!(literal, TokenKind::Term);
                }
                _ => {
                    generate_invalid_token_error!(self, "all test", c.to_string(), c.len_utf8(), 0);
                }
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    macro_rules! result_token {
        ($literal:expr, $kind:expr) => {
            Token {
                literal: $literal,
                kind: $kind,
            }
        };
    }

    fn assert_token<'de>(token: Option<Result<Token<'de>, miette::Error>>, expected: Token<'de>) {
        assert!(token.is_some());
        let token = token.unwrap();
        assert!(token.is_ok());
        let token = token.unwrap();
        assert_eq!(expected, token);
    }

    fn compare_token_vec<'de>(
        expected: Vec<Token<'de>>,
        generated: Vec<Result<Token<'de>, miette::Error>>,
    ) -> bool {
        if expected.len() != generated.len() {
            return false;
        };

        expected.iter().zip(generated.iter()).all(|(e, g)| match g {
            Ok(token) => e == token,
            Err(error) => {
                eprintln!("{error}");
                return false;
            }
        })
    }

    #[test]
    fn test_lex_puntuation() {
        let text = "[].,;:[";
        let lexer = Lexer::new(text);

        let expected_tokens = vec![
            result_token!("[", TokenKind::LBracket),
            result_token!("]", TokenKind::RBracket),
            result_token!(".", TokenKind::Dot),
            result_token!(",", TokenKind::Comma),
            result_token!(";", TokenKind::DotComma),
            result_token!(":", TokenKind::Colon),
            result_token!("[", TokenKind::LBracket),
        ];

        let lexer_tokens: Vec<_> = lexer.collect();
        assert_eq!(compare_token_vec(expected_tokens, lexer_tokens), true);
    }
    #[test]
    fn test_lex_simple_ident() {
        let text = "<#ident>";
        let mut lexer = Lexer::new(text);

        let token = result_token!("ident", TokenKind::Ident);

        let ident_token = lexer.next();
        assert_token(ident_token, token);
    }

    #[test]
    fn test_lex_simple_uri() {
        let text = "<https://aulaglobal.uc3m.es/pluginfile.php/7309413/mod_resource/content/1/T4Agentes2425.pdf>";
        let mut lexer = Lexer::new(text);

        let token = result_token!("https://aulaglobal.uc3m.es/pluginfile.php/7309413/mod_resource/content/1/T4Agentes2425.pdf", TokenKind::URI);

        let ident_token = lexer.next();
        assert_token(ident_token, token);
    }

    #[test]
    fn test_lex_simple_literal() {
        let text = r#""Este texto es falso""#;
        let mut lexer = Lexer::new(text);

        let token = result_token!("Este texto es falso", TokenKind::Literal);

        let ident_token = lexer.next();
        assert_token(ident_token, token);

        assert!(matches!(lexer.next(), None));
    }

    #[test]
    fn test_lex_simple_term() {
        let text = r#"ex:hasAttribute"#;
        let lexer = Lexer::new(text);

        let token = vec![
            result_token!("ex", TokenKind::Term),
            result_token!(":", TokenKind::Colon),
            result_token!("hasAttribute", TokenKind::Term),
        ];

        let ident_token: Vec<_> = lexer.collect();
        assert_eq!(compare_token_vec(token, ident_token), true);
    }
    #[test]
    fn test_lex_basic_prefix() {
        let text = r#"@prefix rr: <http://www.w3.org/ns/r2rml#>."#;
        let lexer = Lexer::new(text);

        let token = vec![
            result_token!("@prefix", TokenKind::Prefix),
            result_token!("rr", TokenKind::Term),
            result_token!(":", TokenKind::Colon),
            result_token!("http://www.w3.org/ns/r2rml#", TokenKind::URI),
            result_token!(".", TokenKind::Dot),
        ];

        let ident_token: Vec<_> = lexer.collect();
        assert_eq!(compare_token_vec(token, ident_token), true);
    }


    #[test]
    fn test_lex_triple() {
        let text = r#"<#ThisMapping> has:attr ox:soma;"#;
        let lexer = Lexer::new(text);

        let token = vec![
            result_token!("ThisMapping", TokenKind::Ident),
            result_token!("has", TokenKind::Term),
            result_token!(":", TokenKind::Colon),
            result_token!("attr", TokenKind::Term),
            result_token!("ox", TokenKind::Term),
            result_token!(":", TokenKind::Colon),
            result_token!("soma", TokenKind::Term),
            result_token!(";", TokenKind::DotComma),
        ];

        let ident_token: Vec<_> = lexer.collect();
        assert_eq!(compare_token_vec(token, ident_token), true);
    }
}
