import calzone
import numpy
from numpy.testing import assert_allclose


def test_stopping_power():
    """Test the stopping power function."""

    p = calzone.Physics()
    s = p.stopping_power(pid=11, material="G4_H", energy=1.0)
    assert isinstance(s, float)
    assert s != 0.0

    energies = numpy.geomspace(1E-01, 1E+01, 21)
    s0 = [
        p.stopping_power(pid=11, material="G4_H", energy=ei) for ei in energies
    ]
    s1 = p.stopping_power(pid=11, material="G4_H", energy=energies)
    assert_allclose(s0, s1)

    energies = numpy.geomspace(1E-01, 1E+01, 21)
    s0 = [
        p.stopping_power(pid=11, material="G4_H", energy=energies[-1],
                         cut=ei) for ei in energies
    ]
    s1 = p.stopping_power(pid=11, material="G4_H", energy=energies[-1],
                          cut=energies)
    assert_allclose(s0, s1)
