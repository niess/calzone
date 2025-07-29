# Calzone  <img src="https://github.com/niess/calzone/blob/master/docs/source/_static/images/logo.svg" height="30px"> [![][RTD_BADGE]][RTD]

Calzone (**CAL**orimeter **ZONE**) is a Python package built on top of
[Geant4][Geant4]. It was developed in the context of geosciences with the
objective of studying the emission of radioactivity from volcanoes [(Terray et
al., 2020)][TGV+20], and in particular to simulate the response of gamma
spectrometers deployed in the field. To this end, Calzone was developed in
conjunction with [Goupil][GOUPIL] [(Niess et al., 2024)][NVT24], a backward
gamma transport engine, and is interoperable with the latter. Yet, both packages
can be used entirely independently, if necessary.

Please refer to the online [documentation][RTD] and the [examples][EXAMPLES] for
further information.


## Installation

Binary distributions of Calzone are available from [PyPI][PyPI], e.g. as

```bash
python -m pip install calzone
```

In addition, Calzone requires 2 GB of [Geant4][Geant4] data tables, which are
not included in the Python package. Once Calzone has been installed, these can
be downloaded as

```bash
python -m calzone download
```

Please refer to the [documentation][RTD_INSTALLATION] for alternative
installation methods.


## Quick start

```toml
# file: geometry.toml

[Environment]
sphere = 1E+05  # cm
material = "G4_WATER"

[Environment.Source]
sphere = 1E+02  # cm
material = "G4_WATER"

[Environment.Source.Detector]
cylinder = { length = 5.1, radius = 2.55 }  # cm
material = "G4_SODIUM_IODIDE"
role = "record_deposits"
```

```python
# file: run.py

import calzone

# Instanciate a simulation engine (using a TOML geometry).
# Requires installing `tomli` for Python < 3.11.
simulation = calzone.Simulation("geometry.toml")

# Generate primary particles.
source = simulation.geometry.find("Source")
energy = 1.0  # MeV
primaries = simulation.particles()  \
    .pid("gamma")                   \
    .energy(energy)                 \
    .inside(source)                 \
    .generate(1000)

# Run the simulation and fetch deposits.
detector = simulation.geometry.find("Detector")
deposits = simulation  \
    .run(primaries)    \
    .deposits[detector.path]
```

## License
The Calzone source is distributed under the **GNU LGPLv3** license. See the
provided [LICENSE][LICENSE] and [COPYING.LESSER][COPYING.LESSER] files.
Additionaly, Calzone uses software developed by Members of the [Geant4][Geant4]
Collaboration, which is under a [specific license][G4_LICENSE].


[COPYING.LESSER]: https://github.com/niess/calzone/blob/master/COPYING.LESSER
[EXAMPLES]: https://github.com/niess/calzone/blob/master/examples
[JSON]: https://www.json.org/json-en.html
[Geant4]: http://cern.ch/geant4
[Goupil]: https://github.com/niess/goupil
[G4_LICENSE]: https://geant4.web.cern.ch/download/license#license
[LICENSE]: https://github.com/niess/calzone/blob/master/LICENSE
[NVT24]: https://doi.org/10.48550/arXiv.2412.02414
[PyPI]: https://pypi.org/project/calzone/
[RTD]: https://calzone.readthedocs.io/en/latest/?badge=latest
[RTD_BADGE]: https://readthedocs.org/projects/calzone/badge/?version=latest
[RTD_INSTALLATION]: https://calzone.readthedocs.io/en/latest/installation.html
[TGV+20]: https://doi.org/10.1029/2019JB019149
[TOML]: https://toml.io/en/
[YAML]: https://yaml.org/
