[![Documentation Status](https://readthedocs.org/projects/calzone/badge/?version=latest)](https://calzone.readthedocs.io/en/latest/?badge=latest)

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


## Installation

Binary distributions of Calzone are available from [PyPI][PyPI], for Linux
`x86_64`, e.g. as

```bash
pip3 install calzone
```

Alternatively, in order to build Calzone from the source, a working
[Geant4][Geant4] installation is required. Please refer to the online
[documentation][INSTALLATION] for further instructions.


## License
The Calzone source is distributed under the **GNU LGPLv3** license. See the
provided [LICENSE](LICENSE) and [COPYING.LESSER](COPYING.LESSER) files.


[INSTALLATION]: https://calzone.readthedocs.io/en/latest/installation.html
[JSON]: https://www.json.org/json-en.html
[Geant4]: https://geant4.web.cern.ch/docs/
[PyPI]: https://pypi.org/project/calzone/
[TOML]: https://toml.io/en/
[YAML]: https://yaml.org/
