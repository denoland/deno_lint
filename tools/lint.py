#!/usr/bin/env python
# Copyright 2018-2020 the Deno authors. All rights reserved. MIT license.

import os
import sys
import argparse
from util import enable_ansi_colors, root_path, run


def main():
    enable_ansi_colors()
    os.chdir(root_path)

    clippy()
    dlint()


def clippy():
    print "clippy"
    args = ["cargo", "clippy", "--all-targets", "--release", "--locked"]
    run(args + ["--", "-D", "clippy::all"], shell=False, quiet=True)


def dlint():
    print "deno lint"
    run(["target/release/examples/dlint", "benchmarks/benchmarks.ts"],
        shell=False, quiet=True)


if __name__ == "__main__":
    sys.exit(main())
