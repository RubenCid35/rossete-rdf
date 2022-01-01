
use rossete_rdf::mappings::{
    maps::Mapping,
    AcceptedType,
    parts::Parts
};
use std::path;
use rossete_rdf::ResultApp;

fn main() -> ResultApp<()>{
    let mut app = Mapping::new(String::from("Test_Mapping"));
    app.add_component(
        Parts::LogicalSource{
            source: path::PathBuf::from("./examples/data/file-1.csv"),
            reference_formulation: AcceptedType::CSV,
            iterator: String::new(),
        }
    );
    app.add_component(
        Parts::SubjectMap{
            components: vec![
                Parts::Template{ template: String::from("http://loc.example.com/latlong/{},{}"), input_fields: vec![String::from("longitude"), String::from("latitude")] },
                Parts::Class(String::from("schema:Coordinates")),
                Parts::GraphMap(Box::new(Parts::Constant("ex:Stop".into())))
            ],
        }
    );

    println!("{:?}", app);
    Ok(())
}