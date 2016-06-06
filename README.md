[![Build Status](https://travis-ci.org/marcmo/enforcer.svg?branch=master)](http://travis-ci.org/marcmo/enforcer) [![Appveyor Build status](https://ci.appveyor.com/api/projects/status/vv4t6mfr25p61a6p?svg=true)](https://ci.appveyor.com/project/marcmo/enforcer)

enforcer
========

check source code for certain metrics (intended as a pre-commit hook)

![Screenshot](https://github.com/marcmo/enforcer/blob/master/doc/enforcerv0.5.gif)

## Usage

    enforcer for code rules

    Usage:
      enforcer [-g ENDINGS...] [-c|--clean] [-q|--quiet] [-t|--tabs] [-l <n>|--length=<n>] [-j <N>|--threads=<N>]
      enforcer (-h | --help)
      enforcer (-v | --version)
      enforcer (-s | --status)

    Options:
      -g ENDINGS        use these file endings (e.g. ".h").
      -h --help         show this screen.
      -v --version      show version.
      -s --status       show configuration status.
      -q --quiet        only count found entries.
      -c --clean        clean up trailing whitespaces and convert tabs to spaces.
      -t --tabs         leave tabs alone (without that tabs are considered wrong).
      -l --length=<n>   max line length [not checked if empty].
      -j --threads=<N>  number of threads [default: 4].

## Example config file (name .enforcer)

    ignore = [".git", ".repo"]
    endings = [".c", ".cpp", ".h"]

## Configuration

If you place a `.enforcer` file with the above content in your project directory, all files ending
in `.c`, `.cpp` and `.h` will be checked. (`.git` and `.repo` directories will be ignored.)
The config file uses the [TOML](https://github.com/toml-lang/toml) format.

## Example Usage

Let's see how we perform on the linux kernel.

    linux-4.5 > find . -type f | wc -l
      52882

    linux-4.5 > enforcer -v
      Version: 0.5.0
    linux-4.5 > time enforcer -t -n
    41100 / 41100 [==================================================] 100.00 % 8337.43/s
    enforcer-error-count: 3655
    checked 41100 files (enforcer_errors!)
      [with ILLEGAL CHARS:1083]
      [with TRAILING SPACES:2572]

    real	0m6.340s
    user	0m3.755s
    sys	0m3.779s

Ok, let's at least remove the trailing whitespaces:

    linux-4.5 > time enforcer -t -c
    ...
    checked 41100 files (enforcer_errors!)
      [with ILLEGAL CHARS:1083]
      [with TRAILING SPACES:2572]

    real	0m5.255s
    user	0m4.207s
    sys	0m5.934s

Now check again

    linux-4.5 > time enforcer -t -q
    41100 / 41100 [==================================================] 100.00 % 18314.68/s
    enforcer-error-count: 1083
    checked 41100 files (enforcer_errors!)
      [with ILLEGAL CHARS:1083]

    real	0m3.621s
    user	0m3.758s
    sys	0m2.411s

