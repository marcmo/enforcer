[![Build Status](https://travis-ci.org/marcmo/enforcer.svg?branch=master)](http://travis-ci.org/marcmo/enforcer) [![Appveyor Build status](https://ci.appveyor.com/api/projects/status/vv4t6mfr25p61a6p?svg=true)](https://ci.appveyor.com/project/marcmo/enforcer)

enforcer
========

check source code for certain metrics (intended as a pre-commit hook)

## Usage

    enforcer for code rules

    Usage:
      enforcer [-g GLOB...] [-c|--clean] [-n|--count]
      enforcer (-h | --help)
      enforcer (-v | --version)
      enforcer (-s | --status)

    Options:
      -g GLOB       use these glob patterns (e.g. \"**/*.h\")
      -h --help     Show this screen.
      -v --version  Show version.
      -s --status   Show configuration status.
      -n --count    only count found entries
      -c --clean    clean up trailing whitespaces

## Example config file (name .enforcer)

    ignore = [".git", ".repo"]
    globs = ["**/*.c", "**/*.cpp", "**/*.h"]

If you place a `.enforcer` file with the above content in your project directory, all files ending
in `.c`, `.cpp` and `.h` will be checked. (`.git` and `.repo` directories will be ignored.)
The config file uses the [TOML](https://github.com/toml-lang/toml) format.

