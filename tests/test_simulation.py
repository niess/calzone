import calzone
import numpy
from numpy.testing import assert_allclose


def test_particles():
    """Test the particles() function."""

    p = calzone.particles(1)
    assert p.size == 1
    assert p["energy"] == 1
    assert p["pid"] == 22

    p = calzone.particles(1, pid="e-")
    assert p["pid"] == 11

    p = calzone.particles(3, energy=(1, 2, 3))
    assert (p["energy"] == (1, 2, 3)).all()


def test_ParticlesGenerator():
    """Test the particles generator."""

    data = { "A": { "box": 1.0, "B": { "box": 0.5 }}}
    simulation = calzone.Simulation(data)
    A = simulation.geometry["A"]
    B = simulation.geometry["A.B"]

    particles = simulation.particles() \
        .direction((1,0,0))            \
        .position((0,1,0))             \
        .energy(2.0)                   \
        .pid("e+")                     \
        .generate(1)

    assert_allclose(particles["direction"], ((1,0,0),))
    assert_allclose(particles["position"], ((0,1,0),))
    assert_allclose(particles["energy"], (2.0,))
    assert_allclose(particles["pid"], (-11,))
    assert_allclose(particles["tid"], (1,))
    assert_allclose(particles["event"], (0,))

    particles = simulation.particles() \
        .on("A", direction="ingoing")  \
        .generate(1)
    assert A.side(particles) == (0,)

    simulation.random.seed = 0
    particles = simulation.particles() \
        .inside("A")                   \
        .generate(100000)
    assert (B.side(particles) <= 0).all()
    assert (particles["tid"] == 1).all()
    assert (particles["event"] == range(particles.size)).all()

    simulation.random.seed = 0
    particles = simulation.particles()       \
        .inside("A", include_daughters=True) \
        .generate(100000)
    assert (B.side(particles) == 1).any()

    simulation.random.seed = 0
    particles = simulation.particles()       \
        .spectrum(((0.5, 0.2), (1.5, 0.8)))  \
        .generate(100000)
    p0 = sum(particles["energy"] == 0.5) / particles.size
    assert abs(p0 - 0.2) <= 3.0 * (p0 * (1 - p0) / particles.size)**0.5


def test_Physics():
    """Test the Physics interface."""

    physics = calzone.Physics()
    assert physics.default_cut == 0.1
    assert physics.em_model == "standard"
    assert physics.had_model == None

    physics = calzone.Physics("penelope")
    assert physics.em_model == "penelope"

    physics = calzone.Physics(had_model="FTFP_BERT")
    assert physics.had_model == "FTFP_BERT"

    simulation = calzone.Simulation()
    simulation.physics = "dna"
    assert simulation.physics.em_model == "dna"


def test_Random():
    """Test the Random interface."""

    rng = calzone.Random()
    assert rng.index == 0

    rng = calzone.Random(1)
    assert rng.seed == 1
    assert rng.index == 0
    v = rng.uniform01(10)

    rng = calzone.Random(1, index=9)
    assert rng.seed == 1
    assert rng.index == 9
    assert rng.uniform01() == v[-1]

    rng.seed = 0
    assert (rng.uniform01(10) == v).all()

    rng.index = 5
    assert (rng.uniform01(5) == v[5:]).all()


def test_Simulation():
    """Test the Simulation interface."""

    # Test setters & getters.
    simulation = calzone.Simulation()
    assert simulation.geometry == None

    data = {"A": {
        "box": 100.0, "material": "G4_WATER", "role": "catch_outgoing"
    }}
    simulation = calzone.Simulation(data)
    assert simulation.geometry is not None
    assert simulation.geometry.root.name == "A"

    # Test reproducibility.
    simulation.random.seed = 0
    particles = simulation.particles()  \
        .inside("A")                    \
        .pid("gamma")                   \
        .powerlaw(0.5, 1.5, exponent=0) \
        .generate(1000)
    result0 = simulation.run(particles)

    events, i = numpy.unique(result0.particles["A"]["event"], return_index=True)
    random_indices = result0.particles["A"][i]["random_index"]
    primaries = particles[events]
    simulation.tracking = True
    result1 = simulation.run(primaries, random_indices=random_indices)
    assert result1.particles["A"].size == result0.particles["A"].size
    result1.particles["A"]["event"] = result0.particles["A"]["event"]
    assert (result1.particles["A"] == result0.particles["A"]).all()

    # Check tracks info.
    sel = result1.tracks["tid"] == 1
    tracks = result1.tracks[sel]
    assert tracks.size == primaries.size
    assert (tracks["event"] == range(tracks.size)).all()
    assert (tracks["pid"] == primaries["pid"]).all()

    # Check vertices info.
    sel = result1.vertices["tid"] == 1
    vertices = result1.vertices[sel]
    for event in range(tracks.size):
        sel = vertices["event"] == event
        vertex = vertices[sel][0]
        assert vertex["energy"] == primaries[event]["energy"]
