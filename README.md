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

## Configuration

If you place a `.enforcer` file with the above content in your project directory, all files ending
in `.c`, `.cpp` and `.h` will be checked. (`.git` and `.repo` directories will be ignored.)
The config file uses the [TOML](https://github.com/toml-lang/toml) format.

## Example Usage

Let's see how we performe on the linux kernel.

    linux-4.5 > find . -type f | wc -l
      52882

    linux-4.5 > enforcer -v
      Version: 0.4.0
    linux-4.5 > time enforcer -t -n
    enforcer-error-count: 3655
    checked 41100 files (enforcer_errors!)
      [with ILLEGAL CHARS:1083]
      [with TRAILING SPACES:2572]

    real	0m24.657s
    user	0m3.011s
    sys	0m4.225s

Ok, let's at least remove the trailing whitespaces:

    linux-4.5 > time enforcer -t -c
    TRAILING_SPACES:[arch/alpha/include/asm/agp.h] -> removing
    ...
    TRAILING_SPACES:[usr/gen_init_cpio.c] -> removing
    checked 41100 files (enforcer_errors!)
      [with ILLEGAL CHARS:1083]
      [with TRAILING SPACES:2572]

    real	1m0.873s
    user	0m3.250s
    sys	0m7.667s

Now check again

    linux-4.5 > enforcer -t -n
    enforcer-error-count: 1083
    checked 41100 files (enforcer_errors!)
      [with ILLEGAL CHARS:1083]

