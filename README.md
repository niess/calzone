# Calzone
(**CAL**orimeter **ZONE**)


## Description

CalZone is a [Geant4][Geant4] Python wrapper for simulating the energy
deposition by high-energy particles in a calorimeter. The interface has been
designed with simplicity in mind. Primary particles are injected into the
simulation volume as a `numpy.ndarray`, and a `numpy.ndarray` of energy deposits
is returned. The Monte Carlo geometry is encoded in a Python `dict` which can be
loaded from configuration files, e.g. using [JSON][JSON], [TOML][TOML] or
[YAML][YAML] formats. This basic workflow is illustrated below,

```python
import calzone

simulation = calzone.Simulation("geometry.toml")
particles = calzone.particles(10000, pid="e-", energy=0.5, position=(0,0,1))
deposits = simulation.run(particles).deposits
```


## License
The Calzone source is distributed under the **GNU LGPLv3** license. See the
provided [LICENSE](LICENSE) and [COPYING.LESSER](COPYING.LESSER) files.


[JSON]: https://www.json.org/json-en.html
[Geant4]: https://geant4.web.cern.ch/docs/
[TOML]: https://toml.io/en/
[YAML]: https://yaml.org/
