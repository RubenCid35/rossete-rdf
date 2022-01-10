
use super::AcceptedType;

pub enum Parts{
    // rr:logicalSource
    LogicalSource{
        source: std::path::PathBuf,
        reference_formulation: AcceptedType,
        iterator: String,
    },
    // rr:subjectMap
    SubjectMap{
        components: Vec<Self>
    },
    // rr:predicateObjectMap
    PredicateObjectMap{
        predicate: String,
        object_map: Vec<Parts>
    },
    // rr:parentTriplesMap
    ParentTriplesMap{
        other_map: String,
        join_condition: [String;2] // Child-Parent
    },
    // rr:graphMaps
    GraphMap(Box<Self>),
    // rr:class
    Class(String),
    // rml:reference
    Reference(String),
    // rr:constant
    Constant(String),
    // rr:dataType
    DataType(String),
    // rr:termType
    TermType(String),
    // rr:template
    Template{
        template: String,
        input_fields: Vec<String>,
    },
    Term(String)
}

impl std::fmt::Debug for Parts{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            Self::LogicalSource{source, reference_formulation, iterator} => {
                writeln!(f, "rml:logicalSource [")?;
                writeln!(f, "\t\trml:source \"{}\";", source.display())?;
                writeln!(f, "\t\trml:referenceFormulation ql:{}", reference_formulation)?;
                writeln!(f, "\t\trml:iterador \"{}\"", iterator)?;
                writeln!(f, "\t];")
            }
            Self::SubjectMap{components} => {
                writeln!(f, "rr:subjectMap: [")?;
                for comp in components{
                    writeln!(f, "\t\t{:?}", &comp)?;
                }
                writeln!(f, "\t];")
            }
            Self::PredicateObjectMap{predicate, object_map} => {
                writeln!(f, "rr:predicateObjectMap [")?;
                writeln!(f, "\t\trr:predicate {}", predicate)?;
                writeln!(f, "\t\trr:objectMap [")?;
                for part in object_map.iter(){
                    writeln!(f, "\t\t\t{:?}", part)?;
                }
                writeln!(f, "\t\t]")?;
                writeln!(f, "\t]")
            }
            Self::ParentTriplesMap{other_map, join_condition} => {
                writeln!(f, "rr:parentTriplesMap <#{}>;", other_map)?;
                if !join_condition.iter().all(|x| x.is_empty()){
                    writeln!(f, "\t\t\trr:joinCondition [")?;
                    writeln!(f, "\t\t\t\trr:child \"{}\";", join_condition[0])?;
                    writeln!(f, "\t\t\t\trr:child \"{}\";", join_condition[1])?;
                    writeln!(f, "\t\t\t]")
                }else{
                    Ok(())
                }
            }
            Self::GraphMap(inside) => {
                writeln!(f, "rr:graphMap {:?}", inside)
            }
            Self::Reference(data) => {
                write!(f, "rml:reference \"{}\"", data)
            }
            Self::Term(data) => {
                write!(f, "{}", data)
            }
            Self::Constant(data) => {
                write!(f, "[rr:constant {}]", data)
            }
            Self::Class(data) => {
                write!(f, "rr:class {}", data)
            }
            Self::TermType(data) => {
                write!(f, "rr:termType {}", data)
            }
            Self::DataType(data) => {
                write!(f, "rr:dataType {}", data)
            }
            Self::Template{template, input_fields} => {
                write!(f, "rr:template \"{}\"", add_input_field(template, input_fields))
            }
        }   
    }
}

impl Parts{
    pub fn get_fields(&self) -> std::collections::HashSet<String>{
        let mut fields = std::collections::HashSet::new();
        match self{
            Parts::LogicalSource {..} => {},
            Parts::SubjectMap { components } => {
                for comp in components{
                    fields.extend(comp.get_fields());
                }
            },
            Parts::PredicateObjectMap { predicate:_ , object_map } => {
                for comp in object_map{
                    fields.extend(comp.get_fields());
                }
            },
            Parts::ParentTriplesMap { other_map: _, join_condition } => {
                if !join_condition[0].is_empty(){
                    fields.insert(join_condition[0].clone());
                }
            }
            Parts::GraphMap(other) => {
                fields.extend(other.get_fields());
            },
            Parts::Class(_) => {},
            Parts::Reference(field) => {    
                fields.insert(field.clone());
            },
            Parts::Constant(_) => {},
            Parts::DataType(_) => {},
            Parts::TermType(_) => {},
            Parts::Template { template: _, input_fields } => {
                fields.extend(input_fields.iter().map(|data| data.clone()));
            },
            Parts::Term(_) => {},
        }
        fields
    }

    // Para comprobar la valided de los mappings,
    pub fn is_subjectmap(&self) -> bool{
        match self{
            Self::SubjectMap{..} => true,
            _ => false
        }
    }
    pub fn is_logicalsource(&self) -> bool{
        match self{
            Self::LogicalSource{..} => true,
            _ => false
        }
    }
    pub fn is_predicate(&self) -> bool{
        match self{
            Self::PredicateObjectMap{..} => true,
            _ => false
        }
    }
}


fn add_input_field(temp: &str, field: &Vec<String>) -> String{
    let matches: Vec<_> = temp.match_indices("{}").collect();
    let mut template = temp.to_string();
    let mut offset = 0; // We are moving the rest of indexes when we add a match
    for (mat, (i, _ )) in matches.iter().enumerate(){
        template.insert_str(*i + offset + 1, &field[mat]);
        offset += field[mat].len();
    }
    template
}
