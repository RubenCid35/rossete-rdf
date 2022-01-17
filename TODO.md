## LIST OF TODO ELEMENTS
### ERRORS
#### Major Errors

##### Minor Errors
- [ ] FIXME: Program needs to eliminate the data files that appear in the configuration JSON and they are not going to be used.

### PARSER TODOS
- [ ] TODO: Add the mapping name in the errors message of the parser. Focus in the subelements.
- [ ] TODO: Add less used keywords(See alse: https://rml.io/specs/rml/#vocabulary)

### GENERAL IMPROVEMENTS
- [ ] Using the changed sqlite crate, maybe it is possible to make that reading thread process its own queries instead of common one.
- [ ] FIXME: Better Comment Capture in the sentences with the mapping declaration and urls.

### USAGE
- [ ] Add CLI Interface. Using CLAP.
- [ ] Iterate over the json and xml files to retrive the data.
- [ ] Take into account JSON and XML Iterator 

### FEATURES
- [ ] Create an example and/or Json Schema of the Configuration to Show