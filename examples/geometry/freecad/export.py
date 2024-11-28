#! /usr/bin/env python3
import argparse
import FreeCAD as fc
import numpy as np
import os
from pathlib import Path
import re


ARGS = None


def export():
    """Export FreeCAD objects to Calzone."""

    if ARGS.label is None:
        return

    doc = fc.openDocument(ARGS.path, hidden=True)

    # Export shapes as STL meshes.
    volumes = {}
    for label in ARGS.label:
        obj = doc.findObjects(Label=label)[0]
        shape = get_shape(obj, label)
        filename = snakify(label)
        path = f"meshes/{filename}.stl"
        dump_stl(shape, path)
        volumes[label] = path

    # Dump the corresponding geometry file.
    lines = [
        f"[{ARGS.envelope}]",
        "envelope = \"box\"",
        "material = \"?\"",
    ]

    for (volume, path) in volumes.items():
        lines += [
            "",
            f"[{ARGS.envelope}.{volume}]",
            f"mesh = {{ path = \"{path}\", units = \"{ARGS.units}\" }}",
            "material = \"?\"",
        ]

    lines.append("")
    lines = os.linesep.join(lines)

    filename = snakify(ARGS.envelope)
    path = ARGS.output_directory / f"{filename}.toml"
    path.parent.mkdir(exist_ok=True, parents=True)

    with path.open("w") as f:
        f.write(lines)

    print(f"dumped {path}")


def snakify(name):
    """Convert a Camel Cased name to snake_cased one."""

    return re.sub(r'(?<!^)(?=[A-Z])', '_', name).lower()


def get_shape(obj, label):
    """Get the shape of a (maybe compound) object."""

    if obj.isDerivedFrom("App::Part"):
        # Fuse the shapes of the compound object.
        shapes = []
        for feature in obj.Group:
            if not feature.isDerivedFrom("Part::Feature"):
                raise NotImplementedError()
            shapes.append(feature.Shape)
        if len(shapes) > 1:
            shape = shapes[0].fuse(shapes[1:])
            print(f"fused Part object '{label}'.")
            return shape
        elif len(shapes) == 1:
            return shapes[0]
        else:
            raise ValueError(f"empty Part object '{label}'")

    elif obj.isDerivedFrom("Part::Feature"):
        return obj.Shape
    else:
        raise NotImplementedError()


def dump_stl(shape, path):
    """Export a Shape object as a STL mesh."""

    path = ARGS.output_directory / path

    # Tessellate the shape.
    vertices, triangles = map(np.array, shape.tessellate(ARGS.tolerance))

    # Export the result as a STL mesh.
    float3 = np.dtype([
        ("x", "f4"),
        ("y", "f4"),
        ("z", "f4"),
    ])
    facet_dtype = np.dtype([
        ("normal",  float3),
        ("vertex1", float3),
        ("vertex2", float3),
        ("vertex3", float3),
        ("control", "u2")
    ])
    data = np.empty(
        len(triangles),
        dtype = facet_dtype
    )
    vertex1 = vertices[triangles[:,0],:]
    data["vertex1"]["x"]= vertex1[:,0]
    data["vertex1"]["y"]= vertex1[:,1]
    data["vertex1"]["z"]= vertex1[:,2]

    vertex2 = vertices[triangles[:,1],:]
    data["vertex2"]["x"]= vertex2[:,0]
    data["vertex2"]["y"]= vertex2[:,1]
    data["vertex2"]["z"]= vertex2[:,2]

    vertex3 = vertices[triangles[:,2],:]
    data["vertex3"]["x"]= vertex3[:,0]
    data["vertex3"]["y"]= vertex3[:,1]
    data["vertex3"]["z"]= vertex3[:,2]

    data["control"] = 0

    v1 = vertex2 - vertex1
    v2 = vertex3 - vertex1
    normal = np.cross(v1, v2)
    normal = (normal.T / np.linalg.norm(normal, axis=1)).T

    data["normal"]["x"] = normal[:,0]
    data["normal"]["y"] = normal[:,1]
    data["normal"]["z"] = normal[:,2]

    header_dtype = np.dtype([
        ("header", "S80"),
        ("size", "u4"),
    ])
    header = np.empty(1, dtype=header_dtype)
    header[0]["header"] = path.name[:79].encode()
    header[0]["size"] = len(data)

    path.parent.mkdir(exist_ok=True, parents=True)
    with path.open("wb") as f:
        f.write(header.data)
        f.write(data.data)

    # Log this action.
    print(f"dumped {path} ({len(data)} facets).")
    sys.stdout.flush()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=export.__doc__)
    parser.add_argument("path",
         help = "path to a FreeCAD document containing the initial model."
    )
    parser.add_argument("-e", "--envelope",
        help = "name of the root envelope volume",
        default = "Detector"
    )
    parser.add_argument("-l", "--label",
        help = "label of an object to export",
        action = "append"
    )
    parser.add_argument("-o", "--output-directory",
        help = "path to output the exported geometry",
        type = Path,
        default = Path(".")
    )
    parser.add_argument("-t", "--tolerance",
        help = "tolerance for the tesselation of shapes (in model units)",
        type = float,
        default = 1.0
    )
    parser.add_argument("-u", "--units",
        help = "length unit of the model",
        default = "mm"
    )

    ARGS = parser.parse_args()
    export()
