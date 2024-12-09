import calzone
from pathlib import Path
from tempfile import TemporaryDirectory
import xml.etree.ElementTree as ET

import numpy
from numpy.testing import assert_allclose


PREFIX = Path(__file__).parent

TMPDIR = TemporaryDirectory()


def test_algorithm():
    """Test meshes algorithm."""

    data = {"A": {"mesh": { "path": str(PREFIX / "assets/cube.obj") }}}

    A = calzone.Geometry(data)["A"]
    assert(A.solid == "G4TessellatedSolid")

    data["A"]["mesh"]["algorithm"] = "bvh"
    A = calzone.Geometry(data)["A"]
    assert(A.solid == "Mesh")

    A = calzone.GeometryBuilder(data, algorithm="voxels").build()["A"]
    assert(A.solid == "G4TessellatedSolid")


def test_Box():
    """Test the box shape."""

    HW = 1.0
    data = { "A": { "box": 2 * HW }}
    geometry = calzone.Geometry(data)
    A = geometry["A"]

    assert A.solid == "G4Box"
    assert_allclose(A.aabb(), [3 * [-HW], 3 * [HW]])
    assert_allclose(A.surface_area, 6 * (2.0 * HW)**2)
    assert_allclose(A.origin(), numpy.zeros(3))
    r0 = { "position": numpy.zeros(3) }
    assert(A.side(r0) == 1)
    r0 = { "position": numpy.full(3, 2 * HW) }
    assert(A.side(r0) == -1)

    data["A"]["box"] = [2 * HW, 4 * HW, 6 * HW]
    expected = HW * numpy.arange(1.0, 4.0)
    A = calzone.Geometry(data)["A"]
    assert_allclose(A.aabb(), [-expected, expected])
    assert A.surface_area == 8 * (
        expected[0] * expected[1] +
        expected[0] * expected[2] +
        expected[1] * expected[2]
    )

    data["A"]["box"] = { "size":  [4 * HW, 6 * HW, 8 * HW]}
    expected = HW * numpy.arange(2.0, 5.0)
    A = calzone.Geometry(data)["A"]
    assert_allclose(A.aabb(), [-expected, expected])


def test_Cylinder():
    """Test the cylinder shape."""

    HW, RADIUS, THICKNESS = 2.0, 1.0, 0.1
    data = { "A": { "cylinder": { "length": 2 * HW, "radius": RADIUS }}}
    geometry = calzone.Geometry(data)
    A = geometry["A"]

    assert A.solid == "G4Tubs"
    expected = numpy.array([RADIUS, RADIUS, HW])
    assert_allclose(A.aabb(), [-expected, expected])
    assert_allclose(A.origin(), numpy.zeros(3))
    S0 = 2 * numpy.pi * RADIUS * (RADIUS + 2 * HW)
    assert_allclose(A.surface_area, S0)
    r0 = { "position": numpy.zeros(3) }
    assert(A.side(r0) == 1)

    data["A"]["cylinder"]["thickness"] = THICKNESS
    A = calzone.Geometry(data)["A"]
    ri = RADIUS - THICKNESS
    assert_allclose(
        A.surface_area,
        S0 + 2 * numpy.pi * ri * (2 * HW - ri)
    )
    assert(A.side(r0) == -1)


def test_Envelope():
    """Test the envelope shape."""

    HW = 1.0
    EPS = 1E-02

    data = { "A": { "B": { "box": 2 * HW }}}
    geometry = calzone.Geometry(data)
    geometry.check()
    A = geometry["A"]

    assert A == geometry.root
    assert A.solid == "G4Box"
    assert_allclose(A.aabb(), [3 * [-(HW + EPS)], 3 * [HW + EPS]])

    shapes = { "box": "G4Box", "cylinder": "G4Tubs", "sphere": "G4Orb" }
    for shape, solid in shapes.items():
        data["A"]["envelope"] = shape
        geometry = calzone.Geometry(data)
        geometry.check()
        assert geometry["A"].solid == solid

    data["A"]["envelope"] = { "padding": HW }
    A = calzone.Geometry(data)["A"]
    assert_allclose(A.aabb(), [3 * [-2 * HW], 3 * [2 * HW]])

    data["A"]["envelope"] = { "padding": [HW, 2 * HW, 3 * HW] }
    expected = HW * numpy.arange(2.0, 5.0)
    A = calzone.Geometry(data)["A"]
    assert_allclose(A.aabb(), [-expected, expected])

    padding = HW * numpy.arange(1.0, 7.0)
    data["A"]["envelope"] = { "padding": padding.tolist() }
    expected = [-(padding[::2] + HW), (padding[1::2] + HW)]
    A = calzone.Geometry(data)["A"]
    assert_allclose(A.aabb(), expected)


def test_Geometry():
    """Test the Geometry interface."""

    data = {"A": {"B": {"box": 1.0}}}
    geometry = calzone.Geometry(data)

    assert geometry["A"] == geometry.root
    assert isinstance(geometry["A"], calzone.Volume)
    assert geometry.find("B").path == geometry["A.B"].path

    try:
        import goupil
    except ImportError:
        pass
    else:
        geometry = geometry.export()
        assert len(geometry.sectors) == 2


def test_GeometryBuilder():
    """Test the geometry builder."""

    data = { "A": {
        "box": 1.0,
        "B": { "sphere": 1.0 },
        "C": { "mesh": str(PREFIX / "assets/cube.obj") },
    }}
    geometry = calzone.GeometryBuilder(data)    \
        .delete("A.B")                          \
        .modify("A", material="G4_Al")          \
        .place({"D": {"box": 0.5}}, mother="A") \
        .move("A.C", "A.D.C")                   \
        .build()

    A = geometry["A"]
    assert(A.daughters == ("A.D",))
    assert(A.material == "G4_Al")

    D = geometry["A.D"]
    assert(D.daughters == ("A.D.C",))
    assert(D.material == "G4_AIR")
    assert(D.solid == "G4Box")


def test_Map():
    """Test the Map interface."""

    z = numpy.full((2, 2), 1.0)
    m = calzone.Map.from_array(z, (-1, 1), (-1, 1))

    assert(m.crs == None)
    assert(m.nx == 2)
    assert(m.ny == 2)

    path = Path(TMPDIR.name) / "cube.png"
    m.dump(path)

    m = calzone.Map(path)
    assert(m.crs == None)
    assert(m.nx == 2)
    assert(m.ny == 2)
    assert((m.z == numpy.ones((2, 2))).all())

    data = {"A": {"mesh": {
        "path": str(path),
        "padding": 2.0,
    }}}
    A = calzone.Geometry(data)["A"]
    assert_allclose(A.surface_area, 6 * 4.0)

    path = Path(TMPDIR.name) / "cube.stl"
    m.dump(path, padding=2.0)

    data["A"]["mesh"] = str(path)
    A = calzone.Geometry(data)["A"]
    assert_allclose(A.surface_area, 6 * 4.0)


def test_Mesh():
    """Test the mesh shape."""

    data = { "A": { "mesh": str(PREFIX / "assets/cube.stl") } }
    geometry = calzone.Geometry(data)
    A = geometry["A"]
    assert_allclose(A.surface_area, 6 * 4.0)
    r0 = { "position": numpy.zeros(3) }
    assert(A.side(r0) == 1)
    r0 = { "position": numpy.full(3, 1.0) }
    assert(A.side(r0) == 0)
    r0 = { "position": numpy.full(3, 2.0) }
    assert(A.side(r0) == -1)

    data = { "A": { "mesh": {
        "path": str(PREFIX / "assets/cube.stl"), "units": "m"
    }}}
    geometry = calzone.Geometry(data)
    A = geometry["A"]
    assert_allclose(A.surface_area, 6 * 4E+04)

    data = { "A": { "mesh": str(PREFIX / "assets/cube.stl") } }
    geometry = calzone.Geometry(data)
    A = geometry["A"]
    assert_allclose(A.surface_area, 6 * 4.0)


def test_meshes():
    """Test named meshes."""

    data = {
        "meshes": {
            "Obj": str(PREFIX / "assets/cube.obj"),
            "Stl": str(PREFIX / "assets/cube.stl"),
        },
        "A": {
            "B": { "mesh": "Stl", "position": [+5.0, 0, 0] },
            "C": { "mesh": "Stl", "position": [-5.0, 0, 0] },
            "D": { "mesh": "Obj" },
        }
    }
    geometry = calzone.Geometry(data)
    geometry.check()

    Obj = calzone.describe(mesh="Obj")
    assert(Obj.references == 1)
    assert(Obj.path == str((PREFIX / "assets/cube.obj").resolve()))
    Stl = calzone.describe(mesh="Stl")
    assert(Stl.references == 2)
    assert(Stl.path == str((PREFIX / "assets/cube.stl").resolve()))


def test_Sphere():
    """Test the sphere shape."""

    RADIUS = 1.0
    data = { "A": { "sphere": RADIUS }}
    geometry = calzone.Geometry(data)
    A = geometry["A"]

    assert A.solid == "G4Orb"
    assert_allclose(A.aabb(), [3 * [-RADIUS], 3 * [RADIUS]])
    assert_allclose(A.surface_area, 4 * numpy.pi * RADIUS**2)
    assert_allclose(A.origin(), numpy.zeros(3))
    r0 = { "position": numpy.zeros(3) }
    assert(A.side(r0) == 1)
    r0 = { "position": numpy.full(3, 2 * RADIUS) }
    assert(A.side(r0) == -1)

    THICKNESS = 0.1
    data = { "A": { "sphere": { "radius": RADIUS, "thickness": THICKNESS }}}
    geometry = calzone.Geometry(data)
    A = geometry["A"]

    assert A.solid == "G4Sphere"
    assert_allclose(A.aabb(), [3 * [-RADIUS], 3 * [RADIUS]])
    assert_allclose(
        A.surface_area,
        4 * numpy.pi * (RADIUS**2 + (RADIUS - THICKNESS)**2)
    )
    assert_allclose(A.origin(), numpy.zeros(3))
    r0 = { "position": numpy.zeros(3) }
    assert(A.side(r0) == -1)
    r0 = { "position": numpy.full(3, 2 * RADIUS) }
    assert(A.side(r0) == -1)


def test_Volume():
    """Test the Volume interface."""

    data = {"A": {
        "material": "G4_Al",
        "B": {"sphere": 1.0, "position": (0.0, 0.0, 1.0)}
    }}
    geometry = calzone.Geometry(data)

    A = geometry["A"]
    assert A.daughters == ("A.B",)
    assert A.mother == None
    assert A.solid == "G4Box"
    assert A.role == None
    assert A.name == "A"
    assert A.path == "A"
    assert (A.origin() == numpy.zeros(3)).all()
    coordinates =  { "position": numpy.array((0.0, 0.0, 1.0)) }
    assert A.side(coordinates) == -1
    assert A.side(coordinates, include_daughters=True) == 1

    B = geometry["A.B"]
    assert B.daughters == tuple()
    assert B.mother == "A"
    assert B.solid == "G4Orb"
    assert B.role == None
    assert B.name == "B"
    assert B.path == "A.B"
    assert (B.origin("A") == (0, 0, 1)).all()

    point = numpy.zeros(3)
    assert_allclose(B.local_coordinates(point), [0.0, 0.0, -1.0])
    points = numpy.zeros((4, 3))
    expected = numpy.tile([0.0, 0.0, -1.0], 4).reshape(points.shape)
    assert_allclose(B.local_coordinates(points), expected)
