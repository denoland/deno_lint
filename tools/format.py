#!/usr/bin/env python
# Copyright 2018-2020 the Deno authors. All rights reserved. MIT license.
import os
import sys
from util import run, root_path


def main():
    os.chdir(root_path)
    rustfmt()
    denofmt()


def rustfmt():
    print("rustfmt")
    run([
        "rustfmt",
        "--check",
        "examples/dlint/main.rs",
    ],
        shell=False,
        quiet=True)
    run([
        "rustfmt",
        "--check",
        "src/lib.rs",
    ],
        shell=False,
        quiet=True)


def denofmt():
    print("deno fmt")
    run([
        "deno",
        "fmt",
        "--check",
        "benchmarks/benchmarks.ts",
    ],
        shell=False,
        quiet=True)


if __name__ == "__main__":
    sys.exit(main())
