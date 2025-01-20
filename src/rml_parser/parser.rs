use super::config::ParseFileConfig;
use super::lex::InvalidTokenFound;
use super::lex::Lexer;
use super::lex::{Token, TokenKind};

use std::collections::HashMap;
use std::fmt::Debug;

use miette::{Diagnostic, NamedSource, Result as MietteResult, SourceSpan};
use thiserror::Error;

use crate::rml::common::TermGenerators;
use crate::rml::common::TermType;
use crate::rml::predicate::JoinCondition;
use crate::rml::predicate::PredicateBuilder;
use crate::rml::predicate::PredicateMap;
use crate::rml::sources::LogicalSource;
use crate::rml::sources::RefFormulation;
use crate::rml::subject::SubjectMap;
use crate::rml::RMLComponent;

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
    // FullTerm(&'de str, &'de str),
    FullTerm(String, String),

    /// Literal Representation. This is a tuple.
    /// Thee first value is the literal and the second is flag for wheter it is a URI or a literal string.
    Literal(&'de str, bool),

    /// ident Representation
    Ident(&'de str),

    /// Term A (rdf:type)
    A,
}

impl<'de> Term<'de> {
    pub fn to_string(&self) -> String {
        match self {
            Term::FullTerm(pre, post) => format!("{}:{}", pre, post),
            Term::Literal(literal, _) => literal.to_string(),
            Term::Ident(ident) => format!("{}", ident),
            Term::A => "rdf:type".to_string(),
        }
    }
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

impl<'de> TermPair<'de> {
    // Retrieve a reference to the predicate term
    fn get_predicate(&self) -> &Term<'de> {
        match self {
            TermPair::TermPair(term, _) => &term,
            TermPair::BlankNode(term, _) => &term,
        }
    }
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
    id: String,

    /// All the pairs token content
    term_pairs: Vec<TermPair<'de>>,

    /// FIle Span that corresponds to this object. This is ideal for the generation of errors.
    span: SourceSpan,
}

/// Quick method that can be used to find the `rdf:type | a` in a list of triples.
fn get_type<'de>(grammar: &'de GrammarPrefix, triples: &'de Vec<TermPair<'de>>) -> Option<&'de Term<'de>> {
    triples.iter().find_map(|term| match term {
        TermPair::TermPair(Term::A, ty) => Some(ty),
        TermPair::TermPair(Term::FullTerm(pre, post), ty) => {
            if grammar.rdf == Some(pre) && *post == "type" {
                Some(ty)
            } else {
                None
            }
        }
        _ => None,
    })
}

/// Determine if an unidentified object is a triples map or not. 
fn is_triple_map<'de>(grammar: &'de GrammarPrefix, mapping: &ObjectMap<'de>) -> bool {
    mapping
        .term_pairs.iter()
        .any(| term | match term.get_predicate() {
            Term::FullTerm(pre, post) => {
                if grammar.rr != pre { return false }
                let post = post.to_lowercase();
                match post.as_str() {
                    "subjectmap" | "logicasource" | "predicateobjectmap" => true,
                    _ => false
                }
            }
            _ => false
        })

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

#[derive(Diagnostic, Debug, Error)]
#[error("Missing Grammar Prefix")]
#[diagnostic(
    code(rml::parser::prefix::missing),
    help("The grammar prefix associated wit the uri: \"{uri}\" is missing, usual prefix: {prefix}")
)]
pub struct MissingGrammarPrefix {
    src: NamedSource<String>,
    uri: &'static str,
    prefix: &'static str,
}

#[derive(Diagnostic, Debug, Error)]
#[error("Missing Field in Object")]
#[diagnostic(
    code(rml::parser::object::missing),
    help("The field {field} is missing or incorrectly field in the object <{id}>")
)]
pub struct MissingFieldError {
    src: NamedSource<String>,

    field: &'static str,
    id: String,

    #[label = "Required Field Location"]
    pub span: SourceSpan,
}

#[derive(Diagnostic, Debug, Error)]
#[error("Missing Field in Object")]
#[diagnostic(
    severity(Warning),
    code(rml::parser::object::missing),
    help("The field {field} is missing or incorrectly field in the object <{id}>")
)]
pub struct MissingFieldWarning {
    src: NamedSource<String>,

    field: &'static str,
    id: String,

    #[label = "Required Field Location"]
    pub span: SourceSpan,
}

// -------------------------------------------------------
// -------------------------------------------------------
// Error Macros
// -------------------------------------------------------
// -------------------------------------------------------

macro_rules! missing_puntuation_warning {
    ($self:ident, $punct:expr, $pos:expr) => {
        if (!$self.config.silent) {
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

    ($self:ident, $token:expr, $literal:expr, $span:ident) => {
        Err(InvalidParserTokenFound {
            src: NamedSource::new(&($self).config.get_file(), $self.whole.to_string()),
            token: $token.into(),
            msg: $literal.to_string(),
            err_span: $span,
        }
        .into())
    };
}

/// Special Macro design to improve legibility of the code by initializing the errors
/// and warnings.
///
/// This error is related to the missing fields in the objects. For instance, the missing `source` field in a logicalSource.
/// The macro allows to treat as a warning or an error.
macro_rules! missing_field {
    ($self: ident, $id: ident, $field: expr, $span: ident) => {
        Err(MissingFieldError {
            src: NamedSource::new(&($self).config.get_file(), $self.whole.to_string()),
            field: $field,
            id: $id.to_string(),
            span: $span
        }.into())
    };

    ($self: ident, $id: ident, $field: expr, $span: ident, warning) => {
        if (!$self.config.silent) {
            let err: miette::Error = Err(MissingFieldWarning {
                src: NamedSource::new(&($self).config.get_file(), $self.whole.to_string()),
                field: $field,
                id: $id.to_string(),
                span: $span
            }.into())
            eprintln!("{err:?}");
        }
    };

}

// -------------------------------------------------------
// -------------------------------------------------------
// Parsing
// -------------------------------------------------------
// -------------------------------------------------------

// Grammar prefixes
const PREFIX_RML: &str = "http://semweb.mmlab.be/ns/rml#";
const PREFIX_QL : &str = "http://semweb.mmlab.be/ns/ql#";
const PREFIX_RR : &str = "http://www.w3.org/ns/r2rml#";
const PREFIX_RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const PREFIX_RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";
const PREFIX_XSD : &str = "http://www.w3.org/2001/XMLSchema#";

/// Prefixes that are related to the grammar and logic of a map.
#[doc(hidden)]
#[derive(Default)]
struct GrammarPrefix<'de> {
    rml: &'de str,
    ql: &'de str,
    rr: &'de str,
    rdf: Option<&'de str>,
    rdfs: Option<&'de str>,
    xsd: Option<&'de str>,
}

impl<'de> GrammarPrefix<'de> {
    /// Create a grammar prefix dictionary using a prefix map
    fn from_prefix_map(prefix_map: &PrefixMap<'de>) -> Self {
        let mut grammar = Self::default();
        for (&prefix, uri) in prefix_map.iter() {
            match uri.as_str() {
                PREFIX_RML => grammar.rml = prefix,
                PREFIX_QL => grammar.ql = prefix,
                PREFIX_RR => grammar.rr = prefix,
                PREFIX_RDF => grammar.rdf = Some(prefix),
                PREFIX_RDFS => grammar.rdfs = Some(prefix),
                PREFIX_XSD => grammar.xsd = Some(prefix),
                _ => continue,
            }
        }
        grammar
    }
}

/// Parser object that gets a file path and generates all the tokens, structs and
/// relevant parts.
pub struct Parser<'de> {
    /// Whole File text
    whole: &'de str,

    /// File and Parsing Configuration,
    config: &'de ParseFileConfig,

    /// Intermediate mapping Objects:
    prefix_map: PrefixMap<'de>,

    /// Vec with all objects maps that are found
    objects: Vec<ObjectMap<'de>>,
}

/// Small Function that attemps to capitalize the first letter in a string.
fn capitalize_term(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// Extracts the predicate term from a term pair. It just returns the prefix and the term as literal strings.
///
/// Arguments:
/// * **term** (TermPair). Term that contains a predicate and some kind of object element (term or node with other pairs).
fn get_predicate<'de>(term: &'de TermPair) -> Option<(&'de str, &'de str)> {
    match term {
        TermPair::TermPair(Term::FullTerm(pre, pos), _) => Some((pre, pos)),
        TermPair::BlankNode(Term::FullTerm(pre, pos), _) => Some((pre, pos)),
        TermPair::TermPair(Term::A, _) => Some(("rdf", "type")),
        _ => None,
    }
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
    fn parse_prefix(&mut self, lexer: &mut Lexer<'de>, prefix: &'de str) -> MietteResult<()> {
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
    fn generate_term(&self, lexer: &mut Lexer<'de>, pre: &Token<'de>) -> MietteResult<Term<'de>> {
        lexer.expected_token(TokenKind::Colon)?;
        let post = lexer.expected_token(TokenKind::Term)?;

        let post_text = post.literal.to_lowercase();
        Ok(Term::FullTerm(pre.literal.to_string(), post_text))
    }

    /// Given the start of a named object, it extracts and parses all the inner components.
    fn extract_object(
        &self,
        lexer: &mut Lexer<'de>,
        ident: &Token<'de>,
        scope: usize,
    ) -> MietteResult<Vec<TermPair<'de>>> {
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
    ///
    /// This method parse the tokens and structures its content into a prefix mapping
    /// and list of all the entity / object with their contens. This steps allows to extract
    /// the structure in the file for futher semantical processing.
    pub fn parse_structures(&mut self) -> MietteResult<()> {
        let mut lexer = Lexer::new(self.config, self.whole);
        while let Some(token) = lexer.next() {
            let token = token?;
            let start_pos = token.position.offset();
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
                        span: SourceSpan::from(start_pos..lexer.get_position()),
                    });
                }

                // normal case.
                TokenKind::Ident => {
                    let term_pairs = self.extract_object(&mut lexer, &token, 0)?;
                    self.objects.push(ObjectMap {
                        id: token.literal.to_string(),
                        term_pairs: term_pairs,
                        span: SourceSpan::from(start_pos..lexer.get_position()),
                    });
                    //
                }
                _ => {
                    return invalid_token!(self, token);
                }
            }
        }

        // println!("\nprefixes:\n{:#?}\n", self.prefix_map);
        // println!("\nobjects :\n{:#?}\n", self.objects);

        Ok(())
    }

    /// check if the grammar prefixes are declared and returns the prefix
    fn check_grammar_prefixes(&self) -> MietteResult<GrammarPrefix<'de>> {
        let grammar = GrammarPrefix::from_prefix_map(&self.prefix_map);
        if grammar.rml.is_empty() {
            return Err(MissingGrammarPrefix {
                src: NamedSource::new(self.config.get_file(), "".to_string()),
                uri: &PREFIX_RML,
                prefix: "rml",
            }
            .into());
        }

        if grammar.ql.is_empty() {
            return Err(MissingGrammarPrefix {
                src: NamedSource::new(self.config.get_file(), "".to_string()),
                uri: &PREFIX_QL,
                prefix: "ql",
            }
            .into());
        }

        if grammar.rr.is_empty() {
            return Err(MissingGrammarPrefix {
                src: NamedSource::new(self.config.get_file(), "".to_string()),
                uri: &PREFIX_RR,
                prefix: "rr",
            }
            .into());
        }

        Ok(grammar)
    }

    // ----------------------------------------------------------------------------------------------
    // ----------------------------------------------------------------------------------------------
    // Semantic Parsing of the File
    // ----------------------------------------------------------------------------------------------
    // ----------------------------------------------------------------------------------------------

    fn parse_logical(&self, grammar: &GrammarPrefix, terms: &ObjectMap<'de>) -> Result<LogicalSource, miette::Error> {
        let mut source = None;
        let mut reference = None;
        let mut iterator = String::new();

        // TODO: no db supported yet
        for term in &terms.term_pairs {
            match term {
                TermPair::TermPair(Term::FullTerm(pre, post), term1) => {
                    if *pre == grammar.rml {
                        match (post.as_str(), term1) {
                            ("source", _) => {
                                if let Term::Literal(text, _) = term1 {
                                    source = Some(text.to_string());
                                }
                            }
                            ("referenceformulation", Term::FullTerm(prefix, form)) => {
                                if *prefix == grammar.ql {
                                    reference = RefFormulation::from_str(form);
                                }
                            }
                            ("iterator", Term::Literal(text, _)) => {
                                iterator = text.to_string();
                            }
                            _ => continue,
                        }
                    }
                }
                _ => continue,
            }
        }
        let id = terms.id.clone();
        let span = terms.span;

        if let Some(source) = source {
            if let Some(reference) = reference {
                let logical = LogicalSource::new(source, iterator, reference);
                return Ok(logical);
            } else {
                return missing_field!(self, id, "rml:referenceformulation", span);
            }
        } else {
            return missing_field!(self, id, "rml:source", span);
        }
    }

    /// Given a list of term pairs, this function generates a subject generator.
    /// The function assumes that the data added terms are inside a subject map. Therefore, the corresponding
    /// checks were not added. This allows working with independent objects (like the ones generated by parsing YARRML)
    /// and inner nodes from a TriplesMap mapping.
    fn parse_subject(&self, grammar: &GrammarPrefix, terms: &ObjectMap<'de>) -> Result<SubjectMap, miette::Error> {
        let mut template: Option<TermGenerators> = None;
        let mut class: Option<TermGenerators> = None;

        for term in &terms.term_pairs {
            match term {
                TermPair::TermPair(Term::FullTerm(pre, post), term1) => {
                    if pre != &grammar.rr {
                        continue;
                    }
                    if *post == "template" {
                        if let Term::Literal(text, _) = term1 {
                            template = Some(TermGenerators::template_from_str(text.to_string(), true));
                        }
                    } else if *post == "class" {
                        if matches!(term1, Term::Literal(_, _) | Term::FullTerm(_, _)) {
                            class = Some(TermGenerators::Constant(term1.to_string(), TermType::IRI, None));
                        }
                    }
                }
                _ => continue,
            }
        }

        if template.is_none() {
            let id = terms.id.clone();
            let span = terms.span;
            return missing_field!(self, id, "rr:template", span);
        }

        let mut subject = SubjectMap::new(template.unwrap());
        if let Some(class) = class {
            subject.add_type(class);
        }
        Ok(subject)
    }

    /// Parse Predicate (rr:PredicateMap). THis object only contains a constant with the predicate.
    fn parse_predicate(
        &self,
        grammar: &GrammarPrefix,
        terms: &ObjectMap<'de>,
    ) -> Result<TermGenerators, miette::Error> {
        let mut predicate = None;
        for term in &terms.term_pairs {
            match term {
                TermPair::TermPair(Term::FullTerm(pre, post), pred_term @ Term::FullTerm(_, _)) => {
                    if pre == &grammar.rr && *post == "constant" {
                        predicate = Some(TermGenerators::Constant(pred_term.to_string(), TermType::Pair, None));
                    }
                }
                TermPair::TermPair(Term::FullTerm(pre, post), Term::Literal(uri, true)) => {
                    if pre == &grammar.rr && *post == "constant" {
                        predicate = Some(TermGenerators::Constant(uri.to_string(), TermType::IRI, None));
                    }
                }
                _ => continue,
            }
        }

        let id = terms.id.clone();
        let span = terms.span;

        if let Some(pred) = predicate {
            return Ok(pred);
        } else {
            return missing_field!(self, id, "rr:constant", span);
        }
    }

    /// Parse Object (rr:ObjectMap). THis object only contains a constant with the predicate.
    fn parse_object(&self, grammar: &GrammarPrefix, terms: &ObjectMap<'de>) -> Result<PredicateMap, miette::Error> {
        let mut has_join = false;

        enum TermTy {
            Const,
            Reference,
            Join,
            None,
        }

        let mut gen_ty: TermTy = TermTy::None;
        let mut term_ty = TermType::Text;
        let mut generator = String::new();
        let mut data_ty = String::new();

        let mut parent = String::new();
        let mut join_child = String::new();
        let mut join_parent = String::new();

        for term in &terms.term_pairs {
            match term {
                TermPair::TermPair(Term::FullTerm(pre, post), pred_term) if *pre == grammar.rr => {
                    if *post == "constant" {
                        gen_ty = TermTy::Const;
                        if let Term::Literal(field, _) = pred_term {
                            term_ty = TermType::Text;
                            generator = field.to_string();
                        } else if let fullterm @ Term::FullTerm(_, _) = pred_term {
                            term_ty = TermType::Pair;
                            generator = fullterm.to_string();
                        } else {
                            let span = terms.span;
                            return invalid_token!(
                                self,
                                format!("{pred_term:?}"),
                                "Only literal terms are valid objects.",
                                span
                            );
                        }
                    } else if *post == "datatype" {
                        data_ty = pred_term.to_string();
                    } else if *post == "parenttriplesmap" {
                        // TODO: implement parsing of join condition
                        has_join = true;
                        gen_ty = TermTy::Join;
                    }
                }
                TermPair::TermPair(Term::FullTerm(pre, post), pred_term) if *pre == grammar.rml => {
                    if *post == "reference" {
                        term_ty = TermType::Text;
                        gen_ty = TermTy::Reference;
                        if let Term::Literal(field, _) = pred_term {
                            generator = field.to_string();
                        } else {
                            let span = terms.span;
                            return invalid_token!(
                                self,
                                format!("{pred_term:?}"),
                                "Only literal terms are valid objects.",
                                span
                            );
                        }
                    }
                }
                TermPair::BlankNode(Term::FullTerm(pre, post), pred_term) => {
                    eprintln!("{term:?}");
                }
                _ => {
                    // eprintln!("{term:?}");
                    continue
                },
            }
        }
        let data_ty = if !data_ty.is_empty() { Some(data_ty) } else { None };
        let term_gen = match gen_ty {
            TermTy::Const => TermGenerators::Constant(generator, term_ty, data_ty),
            TermTy::Reference => TermGenerators::Reference(generator, term_ty, data_ty),
            TermTy::Join => TermGenerators::Undeclared,
            TermTy::None => TermGenerators::Undeclared,
        };

        if has_join {
            let join = Some(JoinCondition{
                child_condition: join_child,
                parent_condition: join_parent,
            });

            Ok(PredicateMap::ByJoin(TermGenerators::Undeclared, String::new(), join))
        } else {
            Ok(PredicateMap::ByField(TermGenerators::Undeclared, term_gen))
        }
    }

    /// Parsing of predicateObjectMap
    fn parse_predicate_object(
        &self,
        grammar: &GrammarPrefix,
        terms: &ObjectMap<'de>,
    ) -> Result<Box<dyn RMLComponent>, miette::Error> {
        let mut object_ref: String = String::from("example");
        let mut predicate_ref: String = String::from("example");
        let mut object: Option<Box<dyn RMLComponent>> = None;
        let mut predicate: Option<Box<dyn RMLComponent>> = None;

        for term in &terms.term_pairs {
            let (pre, post) = get_predicate(term).expect("The predicate must be full term with a prefix and term.");
            if pre != grammar.rr {
                continue;
            } // TODO: maybe add warning of unknown predicate

            // println!("object-term: {}", post);
            match post {
                "predicatemap" => match term {
                    TermPair::TermPair(_, term1) => {
                        predicate_ref = term1.to_string();
                    }
                    TermPair::BlankNode(_, vec) => continue,
                },
                "objectmap" => match term {
                    TermPair::TermPair(_, term1) => {
                        object_ref = term1.to_string();
                    }
                    TermPair::BlankNode(_, vec) => continue,
                },
                "predicate" => {
                    if let TermPair::TermPair(_, object_term) = term {
                        predicate = Some(Box::new(TermGenerators::Constant(
                            object_term.to_string(),
                            TermType::IRI,
                            None,
                        )));
                    }
                }
                "object" => {
                    if let TermPair::TermPair(_, object_term) = term {
                        object = Some(Box::new(TermGenerators::Constant(
                            object_term.to_string(),
                            TermType::IRI,
                            None,
                        )));
                    }
                }
                _ => continue,
            }
        }

        if (predicate.is_none() & predicate_ref.is_empty()) || (object.is_none() & object_ref.is_empty()) {
            let span = terms.span;
            return invalid_token!(
                self,
                "rr:predicate / rr:object",
                "A predicateObjectMap requires at least a `rr:predicateMap` and a `rr:objectMap`",
                span
            );
        }

        let predicate_map = PredicateBuilder::new(predicate_ref, object_ref, predicate, object, None);
        if predicate_map.is_complete() {
            // TODO: build the predicate and return that instead. this option is for full / normal mappings.
        }

        Ok(Box::new(predicate_map))
    }

    /// **Lexical Parsing**
    ///
    /// This method parses the object using the lexical meaning of the terms.
    pub fn parse_semantic(&mut self) -> Result<(), miette::Error> {
        let grammar = self.check_grammar_prefixes()?;
        println!("number of objects: {}", self.objects.len());

        let mut components: HashMap<String, Box<dyn RMLComponent>> = HashMap::with_capacity(self.objects.len());
        for (i, obj) in self.objects.iter().enumerate() {
            if let Some(ty) = get_type(&grammar, &obj.term_pairs) {
                println!("type {:?}", ty);
                if let Term::FullTerm(pre, post) = ty {
                    if pre == &grammar.rr {
                        match post.as_str() {
                            "subjectmap" => {
                                let subject = self.parse_subject(&grammar, &obj)?;
                                println!("[{i:>3}] id: {: >15}\tsubject : {:?}", obj.id, subject);
                                components.insert(obj.id.clone(), Box::new(subject));
                            }
                            "objectmap" => {
                                let object = self.parse_object(&grammar, &obj)?;
                                println!("[{i:>3}] id: {: >15}\tobject : {:?}", obj.id, object);
                                components.insert(obj.id.clone(), Box::new(object));
                            }
                            "predicatemap" => {
                                let predicate = self.parse_predicate(&grammar, &obj)?;
                                println!("[{i:>3}] id: {: >15}\tpredicate : {:?}", obj.id, predicate);
                                components.insert(obj.id.clone(), Box::new(predicate));
                            }
                            "predicateobjectmap" => {
                                let predicate = self.parse_predicate_object(&grammar, &obj)?;
                                println!("[{i:>3}] id: {: >15}\tpredicate-object : {:?}", obj.id, predicate);
                                components.insert(obj.id.clone(), predicate);
                            }
                            "triplesmap" => {
                                println!("[{i:>3}] mapping");
                            }
                            _ => {
                                println!("[{i:>3}] {ty:?}");
                            }
                        }
                    } else if pre == &grammar.rml && *post == "logicalsource" {
                        let source = self.parse_logical(&grammar, &obj)?;
                        println!("[{i:>3}] id: {: >15}\tsource  : {:?}", obj.id, source);
                        components.insert(obj.id.clone(), Box::new(source));
                    }
                }
            } else {
                // detect possible unmarked triples map
                if is_triple_map(&grammar, &obj) {
                    println!("[{i:>3}] mapping");
                } else {
                    // Determine from predicate
                    println!("[{i:>3}]  (other)  {obj:?}");

                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests_structure {
    use super::*;
    use std::collections::hash_map::Entry;

    #[test]
    fn test_prefix_declaration() {
        let text = "@prefix rr: <example.com>.";
        let config = ParseFileConfig::default();

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
        let config = ParseFileConfig::default();

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
        let config = ParseFileConfig::default();

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
