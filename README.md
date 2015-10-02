# enforcer

check source code for certain metrics (intended as a pre-commit hook)

## Usage

    enforcer for code rules

    Usage:
      enforcer [-g GLOB...] [-c|--clean]
      enforcer (-h | --help)
      enforcer (-v | --version)

    Options:
      -g GLOB       use these glob patterns (e.g. \"**/*.h\")
      -h --help     Show this screen.
      -v --version  Show version.
      --count       only count found entries
      -c --clean    clean up trailing whitespaces

## Example config file (name .enforcer)

    ignore = [".git", ".repo"]
    globs = ["**/*.c", "**/*.cpp", "**/*.h"]

If you place a .enforcer file with this content (using the [TOML](https://github.com/toml-lang/toml)
format) all files ending in `.c`, `.cpp` and `.h` will be checked. (`.git` and `.repo` directories will be ignored.)

