from collections import defaultdict
import json
import os
from pathlib import Path
from tempfile import TemporaryDirectory

import calzone

try:
    import tomllib as toml
except ImportError:
    try:
        import tomli as toml
    except ImportError:
        toml = None

try:
    import yaml
except ImportError:
    yaml = None


MATERIALS = {
    "elements": {
        "Rockium": { "Z": 11, "A": 22, "symbol": "Rk" },
        "Calzium": { "Z": 17, "A": 33, "symbol": "Cz" },
    },
    "molecules": {
        "Rk": { "density": 2.65, "state": "solid" },
        "Rk2Cz": { "density": 3.73, "state": "solid" },
        "H2O": { "density": 1.0, "state": "liquid" },
        "Water": {
            "density": 1.0,
            "state": "liquid",
            "composition": { "H": 2, "O": 1 }
        },
    },
    "mixtures": {
        "WetRock": {
            "density": 1.8,
            "state": "solid",
            "composition": { "Rk": 0.8, "H2O": 0.199, "G4_AIR": 0.001 },
        },
        "TestRock": {
            "density": 3.1,
            "state": "solid",
            "composition": { "Rk": 0.3, "Rk2Cz": 0.7 },
        }
    }
}

TMPDIR = TemporaryDirectory()


def dump(obj, path):
    """Dump a dictionary to a config file."""

    path = Path(TMPDIR.name) / path

    if path.suffix == ".json":
        content = json.dumps(obj)

    elif path.suffix == ".toml":
        lines = []

        def recurse(obj, prefix=None):
            for k, v in obj.items():
                if isinstance(v, dict):
                    if prefix is not None:
                        k = f"{prefix}.{k}"
                    lines.append(f"[{k}]")
                    recurse(v, prefix=k)
                else:
                    lines.append(f"{k} = {repr(v)}")

        recurse(obj)
        content = os.linesep.join(lines)

    elif path.suffix == ".yaml":
        lines = []

        def recurse(obj, prefix=""):
            for k, v in obj.items():
                if isinstance(v, dict):
                    lines.append(f"{prefix}{k}:")
                    recurse(v, prefix=prefix + "  ")
                else:
                    lines.append(f"{prefix}{k}: {repr(v)}")

        recurse(obj)
        content = os.linesep.join(lines)

    with open(path, "w") as f:
        f.write(content)

    return path



def test_define():
    """Test defining materials from various sources (to Geant4)."""

    calzone.define(materials=MATERIALS)

    path = dump(MATERIALS, "materials.json")
    calzone.define(materials=path)

    if toml is not None:
        path = dump(MATERIALS, "materials.toml")
        calzone.define(materials=path)

    if yaml is not None:
        path = dump(MATERIALS, "materials.yaml")
        calzone.define(materials=path)


def test_describe():
    """Test materials description (from Geant4)."""

    calzone.define(materials=MATERIALS) # custom materials.

    # Check materials base properties.
    for category in ("molecules", "mixtures"):
        for k, v in MATERIALS[category].items():
            for attr in ("density", "state"):
                desc = calzone.describe(material=k)
                assert getattr(desc, attr) == v[attr]

    # Check molecules composition (defined by mole weight).
    desc0 = calzone.describe(material="G4_WATER")
    desc1 = calzone.describe(material="H2O")
    desc2 = calzone.describe(material="Water")
    assert desc0 == desc1
    assert desc0 == desc2

    # Check mixtures composition (defined by mass weight).
    for mixture, data in MATERIALS["mixtures"].items():
        desc = calzone.describe(material=mixture)
        composition = defaultdict(lambda: 0.0)
        for material, weight in data["composition"].items():
            d = calzone.describe(material=material)
            for k, v in d.composition:
                composition[k] += v * weight

        for k, v in desc.composition:
            assert composition[k] == v

    # Check standard rock.
    desc = calzone.describe(material="StandardRock")
    assert desc.density == 2.65
    assert desc.state == "solid"
    assert desc.composition == [("StandardRock", 1.0),]
