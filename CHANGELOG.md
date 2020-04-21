# Changelog

### [0.15.2] - 04/21/2020
* Format code with cargo fmt nightly
  add raketask to check if all tests and clippy and fmt are correct
* Fix tests: windows tests were broken
  since git did funny things with adding/removing CRLF endings
  now we construct the strings by hand
* Add LineEnding option

### [0.15.1] - 04/20/2020
* Squelsh warnings on windows build

### [0.15.0] - 04/20/2020
* Streamline CI
* Create github actions for cross platform build
  update dependencies
  fix cargo warnings
  tidy up code
* Add CRLF check and conversion
  Check for Windows line endings
* Change info level for encoding errors
  Decrease required error reporting level to Normal for when UTF-8
  sequence can't be decoded,

### [0.14.3] - 07/16/2019
* fixed raketask
