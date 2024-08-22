import argparse
import os

import calzone


def main():
    """Entry point for the CLI."""

    parser = argparse.ArgumentParser(
        prog = "python3 -m calzone",
        description = "Command-line utility for the Calzone package.",
        epilog = "Copyright (C) Université Clermont Auvergne, CNRS/IN2P3, LPCA"
    )

    subparsers = parser.add_subparsers(
        title = "command",
        help = "Command to execute",
        dest = "command"
    )

    config = subparsers.add_parser("config",
        help = "print configuration data."
    )

    config.add_argument("-g", "--geant4-version",
        help = "Geant4 version.",
        action = "store_true",
    )

    config.add_argument("-p", "--prefix",
        help = "Calzone installation prefix.",
        action = "store_true",
    )

    config.add_argument("-v", "--version",
        help = "Calzone version.",
        action = "store_true",
    )

    download = subparsers.add_parser("download",
        help = "download Geant4 data."
    )

    download.add_argument("destination",
        help = "downloaded data destination.",
        nargs = "?"
    )

    download.add_argument("-q", "--quiet",
        help = "mute the download status.",
        action = "store_true"
    )

    args = parser.parse_args()

    if args.command == "config":
        result = []
        if args.geant4_version:
            result.append(calzone.GEANT4_VERSION)
        if args.prefix:
            result.append(os.path.dirname(__file__))
        if args.version:
            result.append(calzone.VERSION)
        if result:
            print(" ".join(result))

    elif args.command == "download":
        calzone.download(
            destination = args.destination,
            verbose = not args.quiet
        )
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
