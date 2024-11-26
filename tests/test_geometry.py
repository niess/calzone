import calzone
from pathlib import Path

import numpy
from numpy.testing import assert_allclose


PREFIX = Path(__file__).parent


def test_Box():
    """Test the box shape."""

    HW = 1.0
    data = { "A": { "box": 2 * HW }}
    geometry = calzone.Geometry(data)
    A = geometry["A"]

    assert A.solid == "G4Box"
    assert_allclose(A.aabb(), [3 * [-HW], 3 * [HW]])
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
