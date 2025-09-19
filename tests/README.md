# Calzone tests

Running the Calzone test suite requires [`pytest`][PYTEST].

By default, the full test suite is executed, requiring Geant4 data (see
[installation instructions][RTD_INSTALLATION]), as well as the
[`calzone-display`][CALZONE_DISPLAY] extension and the [`goupil`][GOUPIL]
package (see [requirements.txt](requirements.txt)).

It is possible to opt-out of specific tests by using `pytest` markers, e.g. as
```
pytest -m "not requires_display and not requires_goupil"
```
Refer to the [pytest.ini](../pytest.ini) file for a list of available markers.

[CALZONE_DISPLAY]: https://github.com/niess/calzone-display
[GOUPIL]: https://github.com/niess/goupil
[PYTEST]: https://docs.pytest.org
[RTD_INSTALLATION]: https://calzone.readthedocs.io/en/latest/installation.html#geant4-data
