import calzone
import numpy
from numpy.testing import assert_allclose


def test_Box():
    """Test the box shape."""

    HW = 1.0
    data = { "A": { "box": 2 * HW }}
    geometry = calzone.Geometry(data)
    A = geometry["A"]

    assert A.solid == "G4Box"
    assert_allclose(A.aabb(), [3 * [-HW], 3 * [HW]])
    assert_allclose(A.origin(), numpy.zeros(3))

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


def test_cylinder():
    """Test the cylinder shape."""

    HW, RADIUS = 2.0, 1.0
    data = { "A": { "cylinder": { "length": 2 * HW, "radius": RADIUS }}}
    geometry = calzone.Geometry(data)
    A = geometry["A"]

    assert A.solid == "G4Tubs"
    expected = numpy.array([RADIUS, RADIUS, HW])
    assert_allclose(A.aabb(), [-expected, expected])
    assert_allclose(A.origin(), numpy.zeros(3))
    assert_allclose(A.surface_area, 2 * numpy.pi * RADIUS * (RADIUS + 2 * HW))

    data["A"]["cylinder"]["thickness"] = 0.1
    A = calzone.Geometry(data)["A"]
    assert_allclose(A.surface_area, 2 * numpy.pi * RADIUS * (RADIUS + 2 * HW))


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
