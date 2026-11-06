import calzone


def test_stopping_power():
    """Test the stopping power function."""

    calzone.describe(material="G4_H")
    p = calzone.Physics()
    s = p.stopping_power(11, "G4_H", 1.0)
    assert s != 0.0
