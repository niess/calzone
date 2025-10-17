---
title: 'Calzone: A Geant4 Python wrapper for the simulation of outdoor particle
detectors'
tags:
  - Python
  - Geant4
authors:
  - name: Valentin Niess
    orcid: 0000-0001-7148-6819
    corresponding: true
    affiliation: 1
  - name: Kinson Vernet
    affiliation: 1
  - name: Luca Terray
    orcid: 0000-0003-4708-0995
    affiliation: "1, 2"
affiliations:
 - name: Université Clermont Auvergne, CNRS, LPCA, F-63000 Clermont-Ferrand, France.
   index: 1
 - name: Université Clermont Auvergne, CNRS, IRD, OPGC, Laboratoire Magmas et Volcans, F-63000 Clermont-Ferrand, France.
   index: 2
date: 3 December 2024
bibliography: paper.bib
---

# Summary

The transport of high-energy particles (e.g. [$\gamma$-rays][GAMMA_RAY]) through
ordinary matter is an inherently stochastic process, with individual collisions
described within the framework of [Quantum Field
Theory](https://en.wikipedia.org/wiki/Quantum_field_theory). The resolution of
such transport problems is facilitated by the use of Monte Carlo methods,
denoted Monte Carlo Particles Transport (MCPT) herein. In particular, the
[Geant4][GEANT4] software [@Allison:2016; @Allison:2006; @Agostinelli:2003] is
an established MCPT `C++` library for simulating the passage of high-energy
particles through matter.

[Calzone][CALZONE] (CALorimeter ZONE) is a MCPT [Python][PYTHON] package built
on top of [Geant4][GEANT4]. It was developed in the context of geosciences with
the objective of studying the emission of radioactivity from volcanoes
[@Terray:2020], and in particular to simulate the response of [gamma][GAMMA]
spectrometers deployed in the field. To this end, [Calzone][CALZONE] was
developed in conjunction with [Goupil][GOUPIL] [@Niess:2024], a backward
[gamma][GAMMA] transport engine, and is interoperable with the latter. Yet, both
packages can be used entirely independently, if necessary.

[Calzone's][CALZONE] interface has been designed with simplicity in mind. Source
particles are injected into the simulation volume as a [NumPY][NUMPY]
[array][NDARRAY] [@Harris:2020], and a [NumPY][NUMPY] [array][NDARRAY] of
collected energy deposits (or particles) is returned. The Monte Carlo geometry
is encoded in a [Python][PYTHON] `dict`, which can be loaded from configuration
files, e.g. using [JSON][JSON], [TOML][TOML] or [YAML][YAML] formats. This basic
workflow is illustrated below,

```python
simulation = calzone.Simulation("geometry.toml")
particles = calzone.particles(
    10000,
    pid="gamma",
    energy=0.5,       # MeV
    position=(0,0,1)  # cm
)
deposits = simulation.run(particles).deposits
```

[Calzone][CALZONE] encourages the use of meshes to describe the Monte Carlo
geometry. Various mesh formats are supported, such as [OBJ][OBJ], [STL][STL],
[GeoTIFF][GEOTIFF] and [Turtle][TURTLE]/[PNG][PNG] [@Niess:2020]. These formats
can be used to encode the components of a detector (exported from a [CAD][CAD]
scheme) or a Digital Elevation Model ([DEM][DEM]) describing the surrounding
terrain. Additionally, [Calzone][CALZONE] features an interactive display
([`calzone-display`][CALZONE_DISPLAY]) that allows users to navigate through the
Monte Carlo geometry and to inspect Monte Carlo tracks (see e.g.
\autoref{fig:display-example}).


# Statement of need

The [Geant4][GEANT4] software was designed as a generic toolkit, with the
capability of being extended using the `C++` inheritance mechanism. The software
is provided under an open-source
[licence](https://geant4.web.cern.ch/download/license) and is subjected to
rigorous [validation][GEANT4_VALIDATION] including comparisons with experimental
data [@Allison:2016]. As a result, [Geant4][GEANT4] is employed in a multitude
of [applications](https://geant4.web.cern.ch/about/#applications), including
high-energy physics (its initial scope) and radiation studies (e.g. for medical
or space sciences).

![
Example of [`calzone-display`][CALZONE_DISPLAY]. The background image comprises
a Digital Elevation Model ([DEM][DEM]) of the [Masaya][MASAYA] volcano, derived
from photogrammetry measurements. The grey box on the volcano ridge
(bottom-left) corresponds to a gamma-spectrometer (located at
[$11.983056^\circ$N, $86.172815^\circ$W][DETECTOR_LOC]), the details of which
are displayed in the top-right insert (using wireframe mode). The superimposed
yellow segments illustrate the trajectory of a photon, originating from the
$1.46\,$MeV emission line of $^{40}$K, simulated with [Calzone][CALZONE] and
[Goupil][GOUPIL] in conjunction.
\label{fig:display-example}
](display-example.png)

However, the generic nature of [Geant4][GEANT4] implies a relatively low-level
`C++` [user interface](https://geant4.kek.jp/Reference/). Thus, a number of
software solutions have been developed on top of [Geant4][GEANT4], providing a
higher-level user interface and extending its functionalities. This is
exemplified by, but not limited to, [Gamos][GAMOS] [@Arce:2014], [Gate][GATE]
[@Jan:2004; @Sarrut:2022], [Geant4Py][GEANT4PY], [Gras][GRAS] [@Santin:2005] and
[Topas][TOPAS] [@Faddegon:2020; @Perl:2012].

In the context of geosciences, we encountered specific issues that were not
addressed by [Geant4][GEANT4], and only partially addressed by some of its
derivatives. Some of these issues, which motivated the development of
[Calzone][Calzone], are discussed hereafter.


# Selected Calzone features

This section outlines a number of key features of the [Calzone][CALZONE]
package, along with the specific issues that these features address.

## Native mesh support

The precision of MCPT computations is contingent upon the accuracy of the
geometry description. In the context of geosciences, the aforementioned geometry
includes the particle detector, which is depicted in a mechanical diagram (using
a [CAD][CAD] software), as well as the study site, which is usually represented
by a Digital Elevation Model ([DEM][DEM]). These data are not natively
understood by [Geant4][GEANT4], requiring transcription. A generic approach is
to delineate volumes of the same material (terrain, sensor, mechanical support,
etc.) using surfaces approximated by triangular meshes. For instance, the
[FreeCAD][FREECAD] software is able to export the detector parts as [STL][STL]
files, which could then be re-read and transcribed into
[`G4TessellatedSolids`][G4TS] (e.g., using [CADMesh][CADMESH] [@Poole:2012]).
[Calzone][CALZONE] streamlines this process by defining a geometry format that
serves as an intermediary. This format uses standard objects, including `dict`,
`float`, `list`, and `str`, and integrates various mesh formats, such as
[OBJ][OBJ], [STL][STL], [GeoTiff][GEOTIFF] and [Turtle][TURTLE]/[PNG][PNG]
[@Niess:2020]. [Calzone][CALZONE] then translates this data into
[Geant4][GEANT4] objects.

## Mesh specialisation

The process of meshing a [DEM][DEM] with triangular facets introduces specific
issues. To optimise the geometry traversal, the [Geant4][GEANT4] software uses a
[voxelisation][VOXEL] algorithm. This method scales poorly for [DEMs][DEM] that
typically comprises millions of nodes (see e.g., [@Niess:2020]), and is
inefficient for long-range particles (such as [$\gamma$][GAMMA] and
[$\mu$][MUON]). Thus, [Calzone][CALZONE] defines a dedicated `Mesh` object that
includes a Bounding Volume Hierarchy ([BVH][BVH]) algorithm (partitioning the
surface of the mesh, rather than its volume). The user may then select the
desired algorithm for each mesh. The default approach is to use a surface
[BVH][BVH] for [DEMs][DEM], while [voxelisation][VOXEL] is used otherwise (i.e.
a [`G4TessellatedSolid`][G4TS]).

## Interoperability with Goupil

A further distinctive feature of MCPT applications in geosciences (such as
[gamma-spectrometry][GAMMA_SPECTROMETRY] and [muography][MUOGRAPHY]) is that the
source largely encompasses the detector, which renders analogue simulations
ineffective. In a typical use case, only a few dozen out of a million of
simulated particles leave a signal in the detector. It is therefore often
necessary to rely on Importance Sampling ([IS][IS]) methods. One effective
method in this context is to backward simulate the transport in the detector's
far environment (see e.g. [@Niess:2018; @Niess:2022]). To this end,
[Calzone][CALZONE] is interoperable with [Goupil][GOUPIL] [@Niess:2024].

## Particles generator

Another point of interest for MCPT applications is the modelling of particle
sources. For this purpose, [Calzone][CALZONE] provides a geometry-aware
[`ParticlesGenerator`][GENERATOR] object, which can, for instance, generate
particles entering a specific geometry volume. Moreover, [Calzone's][CALZONE]
[`ParticlesGenerator`][GENERATOR] consistently provides generation weights,
which are essential for [IS][IS] methods.


# Software architecture

The [Calzone][CALZONE] application was developed in [Rust][RUST], with a [Python
3][PYTHON] user interface (using the [PyO3][PYO3] crate). Interfacing with
[Geant4][GEANT4] was facilitated by the [Cxx](https://crates.io/crates/cxx)
crate. The interactive visualisation was implemented using the
[Bevy](https://bevyengine.org/) game engine.


# Author contributions

An initial `C++` prototype of [Calzone][CALZONE] was developed by K.V. and V.N.
Subsequently, V.N. ported [Calzone][CALZONE] to [Rust][RUST] and extended its
functionalities. L.T. was instrumental in initiating, advising and supervising
this project. All authors contributed to the preparation of this manuscript.


# Acknowledgements

This is contribution no. 726 of the ClerVolc program of the International
Research Center for Disaster Sciences and Sustainable Development of the
University of Clermont Auvergne. In addition, We gratefully acknowledge support
from the Mésocentre Clermont-Auvergne of the Université Clermont Auvergne for
providing computing resources needed for validating this work.


# References


[BVH]: https://en.wikipedia.org/wiki/Bounding_volume_hierarchy
[CAD]: https://en.wikipedia.org/wiki/Computer-aided_design
[CADMESH]: https://github.com/christopherpoole/CADMesh
[CALZONE]: https://github.com/niess/calzone/
[CALZONE_DISPLAY]: https://github.com/niess/calzone-display/
[DEM]: https://geant4.kek.jp/Reference/11.2.0/classG4TessellatedSolid.html
[DETECTOR_LOC]: https://www.google.fr/maps/place/11%C2%B058'59.0%22N+86%C2%B010'22.1%22W/
[FREECAD]: https://www.freecad.org/
[GAMMA]: https://en.wikipedia.org/wiki/Photon
[GAMMA_RAY]: https://en.wikipedia.org/wiki/Gamma_ray
[GAMMA_SPECTROMETRY]: https://en.wikipedia.org/wiki/Gamma_spectroscopy
[GAMOS]: https://fismed.ciemat.es/GAMOS/
[GATE]: http://www.opengatecollaboration.org/
[GEANT4]: https://geant4.web.cern.ch/
[GEANT4_VALIDATION]: https://geant4.web.cern.ch/collaboration/working_groups/physval-wg/
[GEANT4PY]: https://github.com/koichi-murakami/g4python/
[GENERATOR]: https://calzone.readthedocs.io/en/latest/api.html#calzone.ParticlesGenerator
[GEOTIFF]: https://fr.wikipedia.org/wiki/GeoTIFF
[GOUPIL]: https://github.com/niess/goupil/
[GRAS]: https://space-env.esa.int/software-tools/gras/
[G4TS]: https://geant4.kek.jp/Reference/11.2.0/classG4TessellatedSolid.html
[IS]: https://en.wikipedia.org/wiki/Importance_sampling
[JSON]: https://www.json.org/json-en.html
[MASAYA]: https://en.wikipedia.org/wiki/Masaya_Volcano
[MUOGRAPHY]: https://en.wikipedia.org/wiki/Muon_tomography
[MUON]: https://en.wikipedia.org/wiki/Muon
[NDARRAY]: https://numpy.org/doc/2.1/reference/generated/numpy.ndarray.html
[NUMPY]: https://numpy.org/
[OBJ]: https://en.wikipedia.org/wiki/Wavefront_.obj_file
[PNG]: (https://en.wikipedia.org/wiki/PNG
[PYO3]: https://crates.io/crates/pyo3/
[PYTHON]: https://www.python.org/
[RUST]: https://www.rust-lang.org/
[STL]: https://en.wikipedia.org/wiki/STL_(file_format)
[TOML]: https://toml.io/en/
[TOPAS]: https://www.topasmc.org/
[TURTLE]: https://github.com/niess/turtle/
[VOXEL]: https://en.wikipedia.org/wiki/Voxel
[YAML]: https://yaml.org/
