# Changelog

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
