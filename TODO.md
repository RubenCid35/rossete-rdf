## LIST OF TODO ELEMENTS
### ERRORS
#### Major Errors

##### Minor Errors
- [x] FIXME: Program needs to eliminate the data files that appear in the configuration JSON and they are not going to be used.
- [ ] FIXME: Catch Error if Prefix is not given in the map.

### PARSER TODOS
- [x] TODO: Add the mapping name in the errors message of the parser. Focus in the subelements.
- [ ] TODO: Add less used keywords and rare uses of other elements (See alse: https://rml.io/specs/rml/#vocabulary)

### GENERAL IMPROVEMENTS
- [x] Using the changed sqlite crate, maybe it is possible to make that writing thread process its own queries instead of common one. (While reading data it is slower)
- [x] FIXME: Better Comment Capture in the sentences with the mapping declaration and urls.

### USAGE
- [ ] Add CLI Interface. Using CLAP.
- [ ] Iterate over the XML files to retrive the data.

### FEATURES
- [ ] Create an example and/or Json Schema of the Configuration to Show