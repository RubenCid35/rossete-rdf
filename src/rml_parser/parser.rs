
use crate::ResultApp;
use crate::errors::ApplicationErrors;
use crate::{info, warning, error};
use super::tokenize::Token;


use crate::mappings::parts::Parts;
use crate::mappings::maps::Mapping;
use crate::mappings::AcceptedType;

use std::collections::HashMap;

fn parse_tokens(stream_tokens: Vec<Token>) -> ResultApp<Vec<Mapping>> {
    let mut prefix_dict: HashMap<String, String> = HashMap::new();
    let mut token_iter = stream_tokens.iter();


    Ok(Vec::new())   
}