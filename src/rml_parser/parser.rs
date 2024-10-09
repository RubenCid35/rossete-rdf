use super::config::ParseFileConfig;
use super::lex::InvalidTokenFound;
use super::lex::Lexer;
use super::lex::{Token, TokenKind};

use std::collections::HashMap;
use std::fmt::Debug;

use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

// -------------------------------------------------------
// -------------------------------------------------------
// Parse-Object-AST
// -------------------------------------------------------
// -------------------------------------------------------

/// Map with all the prefix declarations.
pub type PrefixMap<'de> = HashMap<&'de str, String>;

#[derive(Clone)]
pub enum Term<'de> {
    /// Simple term representation. ej ex:opt -> FullTerm(ex, opt)
    FullTerm(&'de str, &'de str),

    /// Literal Representation. This is a tuple.
    /// Thee first value is the literal and the second is flag for wheter it is a URI or a literal string.
    Literal(&'de str, bool),

    /// ident Representation
    Ident(&'de str),

    /// Term A (rdf:type)
    A,
}

impl<'de> Debug for Term<'de> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Term::FullTerm(pre, post) => write!(f, "{}:{}", pre, post),
            Term::Literal(literal, is_uri) => {
                if *is_uri {
                    write!(f, "<{}>", literal)
                } else {
                    write!(f, "{:?}", literal)
                }
            }
            Term::Ident(ident) => write!(f, "{}", ident),
            Term::A => write!(f, "rdf:type"),
        }
    }
}

pub enum TermPair<'de> {
    /// Basic Term Pair. Ejem: `rml:reference "longitude"`
    TermPair(Term<'de>, Term<'de>),

    /// Pair of a Full Term and a Scope context. One example of it will be:
    /// ```ttl
    ///rr:objectMap [
    ///   rml:reference "longitude"
    ///]
    /// ```
    BlankNode(Term<'de>, Box<Vec<TermPair<'de>>>),
}

impl<'de> Debug for TermPair<'de> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TermPair::BlankNode(term_pair, inside) => {
                writeln!(f, "{term_pair:?} [")?;
                for _in in inside.iter() {
                    writeln!(f, "\t{_in:?}")?;
                }
                write!(f, "]")
            }
            Self::TermPair(predicate, object) => {
                write!(f, "{predicate:?} {object:?}")
            }
        }
    }
}

/// Representation of node in the mapping  file. THis node may be logicalSource, subject, predicate or mappings.
/// This representation will allow the program to implement and use mappings that generated from YARRRML.
#[derive(Debug)]
pub struct ObjectMap<'de> {
    /// associated ident to object in the mapping
    #[allow(dead_code)]
    id: String,

    /// All the pairs token content
    #[allow(dead_code)]
    term_pairs: Vec<TermPair<'de>>,
}

// -------------------------------------------------------
// -------------------------------------------------------
// Error handler
// -------------------------------------------------------
// -------------------------------------------------------

#[derive(Debug, Error, Diagnostic)]
#[error(" A \"{}\" is missing in this position", .token)]
#[diagnostic(
    code(rml::parser::puntuation),
    help("There is a missing puntuation element in this position."),
    severity(Warning)
)]
struct MissingPunctuationWarning {
    pub token: char,

    #[source_code]
    src: NamedSource<String>,

    #[label = "Add a \"{token}\" in this position"]
    err_span: SourceSpan,
}

#[derive(Debug, Error, Diagnostic)]
#[error("{} is incomplete", .ident)]
#[diagnostic(
    code(rml::parser::object::incomplete),
    help("Add at least a `rdf:type` declaration."),
    severity(Warning)
)]
struct InvalidObjectMapDeclaration {
    pub ident: String,

    #[source_code]
    src: NamedSource<String>,

    #[label = "Add at least one  predicate-object pair for this object."]
    err_span: SourceSpan,
}

/// Location and Identification of an invalid token.
#[derive(Diagnostic, Debug, Error)]
#[error("invalid token '{token}' was found.")]
#[diagnostic(
    code(rml::parser::token),
    help("Invalid token was found. Consider check if the mapping is correct.")
)]
pub struct InvalidParserTokenFound {
    #[source_code]
    pub src: NamedSource<String>,

    pub token: String,
    pub msg: String,

    #[label = "{msg}"]
    pub err_span: SourceSpan,
}

// -------------------------------------------------------
// -------------------------------------------------------
// Error Macros
// -------------------------------------------------------
// -------------------------------------------------------

macro_rules! missing_puntuation_warning {
    ($self:ident, $punct:expr, $pos:expr) => {
        if (! $self.config.silent) {
            let file_name = &($self).config.get_file();
            let warning: miette::Error = MissingPunctuationWarning {
                src: NamedSource::new(file_name, $self.whole.to_string()),
                token: $punct,
                err_span: SourceSpan::from(($pos)..($pos + 1)),
            }
            .into();
    
            eprintln!("{warning:?}");
        }
    };
}

macro_rules! invalid_token {
    ($self:ident) => {
        Err(InvalidEndOfFile {
            file_name: $self.config.file_path.clone(),
        }
        .into())
    };
    ($self:ident, $token:ident) => {
        Err(InvalidTokenFound {
            src: NamedSource::new(&($self).config.get_file(), $self.whole.to_string()),
            token: $token.literal.to_string(),
            err_span: SourceSpan::from(($token.position.offset())..($token.get_end())),
        }
        .into())
    };

    ($self:ident, $token:ident, $literal:expr) => {
        Err(InvalidParserTokenFound {
            src: NamedSource::new(&($self).config.get_file(), $self.whole.to_string()),
            token: $token.literal.to_string(),
            msg: $literal.to_string(),
            err_span: SourceSpan::from(($token.position.offset())..($token.get_end())),
        }
        .into())
    };

    ($self:ident, $error:ident, $literal:expr, $span:expr) => {
        Err($error {
            src: NamedSource::new(&($self).config.get_file(), $self.whole.to_string()),
            token: $literal,
            err_span: SourceSpan::from($span),
        }
        .into())
    };
}

// -------------------------------------------------------
// -------------------------------------------------------
// Parsing
// -------------------------------------------------------
// -------------------------------------------------------

/// Parser object that gets a file path and generates all the tokens, structs and
/// relevant parts.
pub struct Parser<'de> {
    /// Whole File text
    whole: &'de str,

    /// File and Parsing Configuration,
    config: &'de ParseFileConfig,

    /// Intermediate mapping Objects:
    prefix_map: PrefixMap<'de>,

    // TODO: maybe change to hashmap?
    /// Vec with all objects maps that are found
    objects: Vec<ObjectMap<'de>>,
}

impl<'de> Parser<'de> {
    /// Generate a new parser object from the configuration
    /// In this method, the lexer is not initialized.
    pub fn new(config: &'de ParseFileConfig, file_content: &'de str) -> Self {
        Self {
            whole: file_content,
            config: config,
            prefix_map: HashMap::new(),
            objects: Vec::with_capacity(2),
        }
    }

    /// Parses the final part of prefix declaration. This part corresponds
    /// to the URI and the final dot. This procedure is common in the case
    /// of `@prefix` and `@base`.
    fn parse_prefix(&mut self, lexer: &mut Lexer<'de>, prefix: &'de str) -> Result<(), miette::Error> {
        // get URI
        let prefix_uri = lexer.expected_token(TokenKind::URI)?;
        let last_pos = prefix_uri.position.offset() + prefix_uri.position.len();

        // save
        self.prefix_map.insert(prefix, prefix_uri.literal.to_string());

        // Determine if there is a dot a the end else return a warning.
        let is_dot = lexer.peek_expected_token(TokenKind::Dot)?;
        if is_dot {
            lexer.next().unwrap()?;
        } else {
            missing_puntuation_warning!(self, '.', last_pos);
        };

        Ok(())
    }

    /// Creates a full term from a pair of term tokens and a colon. It fails if there is no colon in the middle.
    fn generate_term(&self, lexer: &mut Lexer<'de>, pre: &Token<'de>) -> Result<Term<'de>, miette::Error> {
        lexer.expected_token(TokenKind::Colon)?;
        let post = lexer.expected_token(TokenKind::Term)?;
        Ok(Term::FullTerm(pre.literal, post.literal))
    }

    /// Given the start of a named object, it extracts and parses all the inner components.
    fn extract_object(
        &self,
        lexer: &mut Lexer<'de>,
        ident: &Token<'de>,
        scope: usize,
    ) -> Result<Vec<TermPair<'de>>, miette::Error> {
        let mut predicate: Option<Term<'_>> = None;
        let mut scoped = vec![];

        let mut is_closing = false;
        let mut end_triple = false;

        loop {
            let token = if scope == 0 && predicate.is_none() {
                match lexer.next() {
                    Some(t) => t?,
                    None => {
                        missing_puntuation_warning!(self, '.', self.whole.len());
                        break;
                    }
                }
            } else {
                lexer.next_filtered()?
            };

            if is_closing {
                if matches!(token.kind, TokenKind::RBracket) {
                    break;
                } else {
                    return invalid_token!(self, token);
                }
            }
            if end_triple {
                if matches!(token.kind, TokenKind::DotComma) {
                    end_triple = false;
                    continue;
                } else if !matches!(token.kind, TokenKind::RBracket | TokenKind::Dot) {
                    missing_puntuation_warning!(self, ';', token.position.offset() - 1);
                }
            }

            end_triple = false;
            match token.kind {
                TokenKind::Dot => {
                    if scope != 0 {
                        is_closing = true;
                    } else {
                        break;
                    }
                }

                TokenKind::LBracket => {
                    if let Some(ref pred_token) = predicate {
                        let blank_content = self.extract_object(lexer, ident, scope + 1)?;
                        scoped.push(TermPair::BlankNode(pred_token.clone(), Box::new(blank_content)));
                        predicate = None;
                        end_triple = true;
                    } else {
                        return invalid_token!(self, token, "Add a predicate before this token.");
                    }
                }

                TokenKind::RBracket => {
                    if predicate.is_none() {
                        missing_puntuation_warning!(self, '.', token.position.offset() - 1);
                        break;
                    } else {
                        return invalid_token!(self, token);
                    }
                }

                TokenKind::A => match predicate {
                    Some(_) => {
                        return invalid_token!(self, token);
                    }
                    None => {
                        predicate = Some(Term::A);
                    }
                },

                TokenKind::Term => {
                    let term = self.generate_term(lexer, &token)?;
                    if let Some(ref pred_token) = predicate {
                        scoped.push(TermPair::TermPair(pred_token.clone(), term));

                        let is_comma = lexer.peek_expected_token(TokenKind::Comma)?;
                        if !is_comma {
                            predicate = None;
                            end_triple = true;
                        } else {
                            lexer.next();
                        }
                    } else {
                        predicate = Some(term);
                    }
                }

                TokenKind::Ident => {
                    if let Some(ref pred_token) = predicate {
                        let term = Term::Ident(token.literal);
                        scoped.push(TermPair::TermPair(pred_token.clone(), term));
                        predicate = None;
                        end_triple = true;
                    } else {
                        return invalid_token!(self, token, "Add a predicate before this token.");
                    }
                }

                TokenKind::URI => {
                    let term = Term::Literal(&token.literal, true);
                    if let Some(ref term_pred) = predicate {
                        scoped.push(TermPair::TermPair(term_pred.clone(), term));

                        let is_comma = lexer.peek_expected_token(TokenKind::Comma)?;
                        if !is_comma {
                            predicate = None;
                            end_triple = true;
                        } else {
                            lexer.next();
                        }
                    } else {
                        predicate = Some(term);
                    }
                }
                TokenKind::Literal => {
                    if let Some(ref term_pred) = predicate {
                        let term = Term::Literal(&token.literal, false);
                        scoped.push(TermPair::TermPair(term_pred.clone(), term));
                        predicate = None;
                        end_triple = true;
                    } else {
                        return invalid_token!(self, token, "Add a predicate before this token.");
                    }
                }

                _ => {
                    return invalid_token!(self, token);
                }
            }
        }

        if scoped.len() == 0 && scope == 0 {
            return Err(InvalidObjectMapDeclaration {
                ident: ident.literal.to_string(),
                err_span: ident.position.clone(),
                src: NamedSource::new(self.config.get_file(), self.whole.to_string()),
            }
            .into());
        }
        Ok(scoped)
    }

    /// **Parse Structural Layer**
    /// This method parse the tokens and structures its content into a prefix mapping
    /// and list of all the entity / object with their contens. This steps allows to extract
    /// the structure in the file for futher semantical processing.
    pub fn parse_structures(&mut self) -> Result<(), miette::Error> {
        let mut lexer = Lexer::new(self.config, self.whole);
        while let Some(token) = lexer.next() {
            let token = token?;

            match token.kind {
                // prefix definition
                TokenKind::Prefix => {
                    let prefix = lexer.expected_token(TokenKind::Term)?.literal;
                    lexer.expected_token(TokenKind::Colon)?;
                    self.parse_prefix(&mut lexer, prefix)?;
                }
                TokenKind::Base => {
                    self.parse_prefix(&mut lexer, "")?;
                }

                // object creation
                // This is the special case that appears while converting from YARRML
                TokenKind::Term => {
                    let term_full = self.generate_term(&mut lexer, &token)?;
                    let term_pairs = self.extract_object(&mut lexer, &token, 0)?;
                    self.objects.push(ObjectMap {
                        id: format!("{:?}", term_full),
                        term_pairs: term_pairs,
                    });
                }

                // normal case.
                TokenKind::Ident => {
                    let term_pairs = self.extract_object(&mut lexer, &token, 0)?;
                    self.objects.push(ObjectMap {
                        id: token.literal.to_string(),
                        term_pairs: term_pairs,
                    });
                    //
                }
                _ => {
                    return invalid_token!(self, token);
                }
            }
        }

        println!("\nprefixes:\n{:#?}\n", self.prefix_map);
        println!("\nobjects :\n{:#?}\n", self.objects);

        Ok(())
    }
}

#[cfg(test)]
mod tests_structure {
    use super::*;
    use std::collections::hash_map::Entry;
    use std::path::PathBuf;

    #[test]
    fn test_prefix_declaration() {
        let text = "@prefix rr: <example.com>.";
        let config = ParseFileConfig {
            file_path: PathBuf::new(),
            silent: true,
        };

        let mut parser = Parser::new(&config, text);
        assert_eq!(parser.parse_structures().is_ok(), true);
        assert!(matches!(parser.prefix_map.entry("rr"), Entry::Occupied(_)));

        let text = "@prefix rr: dsadsadsa";
        let mut parser = Parser::new(&config, text);
        assert_eq!(parser.parse_structures().is_ok(), false);
    }

    #[test]
    fn test_base_declaration() {
        let text = "@base <example.com>.";
        let config = ParseFileConfig {
            file_path: PathBuf::new(),
            silent: true,
        };

        let mut parser = Parser::new(&config, text);
        assert_eq!(parser.parse_structures().is_ok(), true);
        assert!(matches!(parser.prefix_map.entry(""), Entry::Occupied(_)));

        // added colon in the base declaration.
        let text = "@base : dsadsadsa";
        let mut parser = Parser::new(&config, text);
        assert_eq!(parser.parse_structures().is_ok(), false);
    }

    #[test]
    fn test_simple_object() {
        let text = "<#ident> a rr:TriplesMap.";
        let config = ParseFileConfig {
            file_path: PathBuf::new(),
            silent: true,
        };

        let mut parser = Parser::new(&config, text);
        assert_eq!(parser.parse_structures().is_ok(), true);
        assert_eq!(parser.objects.len(), 1);
        assert_eq!(&parser.objects[0].id, "ident");
        assert!(matches!(
            &parser.objects[0].term_pairs[0],
            TermPair::TermPair(Term::A, Term::FullTerm(_, _))
        ));

        let text = r#"
            <#ident> a rr:TriplesMap;
                rr:logicalSource [
                    rr:source "this map"
                ].
        "#;
        let mut parser = Parser::new(&config, text);
        assert_eq!(parser.parse_structures().is_ok(), true);
        assert_eq!(parser.objects.len(), 1);
        assert_eq!(&parser.objects[0].id, "ident");
        assert_eq!(parser.objects[0].term_pairs.len(), 2);

        if let TermPair::BlankNode(Term::FullTerm(_, _), inner) = &parser.objects[0].term_pairs[1] {
            assert_eq!(inner.len(), 1);
        } else {
            assert!(false);
        }

        // missing dot
        let text = r#"
            <#ident> a rr:TriplesMap;
                rr:logicalSource [
                    rr:source "this map"
                ]
        "#;
        let mut parser = Parser::new(&config, text);
        assert_eq!(parser.parse_structures().is_ok(), true);
    }
}
