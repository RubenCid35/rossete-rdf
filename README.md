# ROSSETE RDF

This application is intended to convert the data stored in a database/CSV/JSON/XML/etc to RDF using a RML mapping file or list of them.
<br>

# Usage

To run this application, you need to run the following CLI command.
```
    rossete-rdf.exe [FLAGS] [OPTIONS] --output <OUTPUT> --MAPPINGS <MAPPINGS>
```

For more information, you can use the help flag (-h|--help). If you use it,
the following prompt will appear with the possible custom usage options and others.
```
Rossete RDF Materisaliser 0.1.0
Rubén Cid Costa
This application is intended to generate a rdf file from one or varios datasets in different formats and a RML mapping.

USAGE:
    rossete-rdf.exe [FLAGS] [OPTIONS] --mappings <MAPPINGS> --output <OUTPUT>

FLAGS:
    -w, --clear      Delete the database if it was created while reading the databases
    -c, --close      If active; the files used are closed.
    -d, --debug      Set the debug mode. It displays more information in the intermediarry parts
    -h, --help       Displays this message
    -V, --version    Prints version information

OPTIONS:
        --config <FILE>          Sets a custom config file to create the main settings of the program
        --mappings <MAPPINGS>    Used mapping in the process of generated rdf. Values: Folder or a file
        --output <OUTPUT>        File name where the output file is written
```
## Requirements
To use this executable, you need to install rust and cargo in your computer, so you can compile this repository.
To check if you have them, use this commands:
```
   rustc --version  // To check if you have Rust Programming Language Installed
   cargo --version  // To check if you have Cargo Package Manager Installed
```

To compile it, you can use the following command in  your terminal and you have Rust and Cargo Installed. 

```
cargo build --release // This will create an executable in the newly created target/release folder
```
You can take the binary as a standalone binary and move whereever you want to.

<br>

# Supported Formats at the current moment
This are the input data files formats and the  output formats that are being addeded to the app and the ones
that are already in.

## Supported Input Formats

| Format           | Working in Progress  | Ready     |
|------------------|-----------|-----------|
| MySQL Database   |  &#x2613; |  &#x2613; |    
| SQLite Database  |  &#x2613; |  &#x2613; |    
| JSON format      |  &#x2611; |  &#x2613; |
| XML Format       |  &#x2613; |  &#x2613; |
| CSV Format       |  &#x2611; |  &#x2611; |
| TSV Format       |  &#x2611; |  &#x2611; |
| Others           |  &#x2613; |  &#x2613; |

## Supported Output Formats

| Format           | Working in Progress   | Ready     |
|------------------|-----------|-----------|
| Turttle          |  &#x2613; |  &#x2613; |    
| TriplesMap       |  &#x2611; |  &#x2613; |    
| Yarm             |  &#x2613; |  &#x2613; |

# Autors:

- Rubén Cid Costa
