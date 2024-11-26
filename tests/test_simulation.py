import calzone
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

    particles = simulation.particles() \
        .on("A", direction="ingoing")  \
        .generate(1)
    assert A.side(particles) == (0,)

    simulation.random.seed = 0
    particles = simulation.particles() \
        .inside("A")                   \
        .generate(100000)
    assert (B.side(particles) <= 0).all()

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
    """Test the physics interface."""

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
